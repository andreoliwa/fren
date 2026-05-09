//! Directory merge.
//!
//! Port of Python's `merge_directories()` from `src/fren.py`. For each
//! source dir, walks recursively and moves each file into the target's
//! mirrored subtree. If a target file already exists, appends a `_Copy`
//! / `_Copy1` / `_Copy2` suffix to the stem until a free name is found.
//!
//! Uses the literal `_Copy` suffix for compatibility with the original
//! Python tool's behavior. A configurable conflict template may replace
//! this in the future.

use crate::FrenError;
use regex::Regex;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const IGNORE_FILES_ON_MERGE: &[&str] = &[".DS_Store"];

/// Merge `sources` into `target`. Files already in `target` get a
/// `_Copy{N}` suffix appended to their stem. Directories are created
/// as needed. `.DS_Store` and similar metadata files are skipped.
///
/// `dry_run = true` walks and reports what would be moved without
/// touching the filesystem.
///
/// Returns the count of files moved (or that would be moved in dry-run).
pub fn merge_directories(
    target: &Path,
    sources: &[&Path],
    dry_run: bool,
) -> Result<MergeReport, FrenError> {
    if !target.is_dir() {
        return Err(FrenError::InvalidInput(format!(
            "target is not a directory: {}",
            target.display()
        )));
    }

    let mut moved: Vec<MergeMove> = Vec::new();
    // For dry-run: track destinations chosen so far so successive
    // would-be-moves don't all collide on the same path.
    let mut planned_destinations: HashSet<PathBuf> = HashSet::new();

    for &source in sources {
        if !source.is_dir() {
            return Err(FrenError::InvalidInput(format!(
                "source is not a directory: {}",
                source.display()
            )));
        }

        for path in walk_files(source)? {
            let stem = path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            if IGNORE_FILES_ON_MERGE.iter().any(|n| *n == stem) {
                continue;
            }
            let rel = path.strip_prefix(source).map_err(|_| {
                FrenError::InvalidInput(format!(
                    "path {} not under source {}",
                    path.display(),
                    source.display()
                ))
            })?;
            let dest_naive = target.join(rel);
            let dest = if dry_run {
                unique_file_name_avoiding(&dest_naive, &planned_destinations)
            } else {
                unique_file_name(&dest_naive)
            };
            planned_destinations.insert(dest.clone());
            moved.push(MergeMove {
                from: path.clone(),
                to: dest.clone(),
            });
            if !dry_run {
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent).map_err(|source| FrenError::Io {
                        path: parent.to_path_buf(),
                        source,
                    })?;
                }
                std::fs::rename(&path, &dest).map_err(|source| FrenError::Io {
                    path: path.clone(),
                    source,
                })?;
            }
        }
    }

    Ok(MergeReport { moved })
}

/// Like [`unique_file_name`] but also avoids any path in `taken`. Used by
/// dry-run mode where filesystem state doesn't yet reflect prior moves
/// in the same batch.
fn unique_file_name_avoiding(path: &Path, taken: &HashSet<PathBuf>) -> PathBuf {
    let mut current = path.to_path_buf();
    while current.exists() || taken.contains(&current) {
        let stem = current
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let ext = current
            .extension()
            .map(|e| e.to_string_lossy().into_owned());

        let (orig_stem, next_idx) = match unique_re().captures(&stem) {
            Some(caps) => {
                #[allow(clippy::expect_used)]
                let orig = caps
                    .name("orig")
                    .expect("regex named group orig")
                    .as_str()
                    .to_string();
                let idx_next = caps
                    .name("idx")
                    .and_then(|m| m.as_str().parse::<u32>().ok())
                    .map_or(1, |n| n + 1);
                (orig, idx_next)
            }
            None => (stem, 0),
        };
        let suffix_num = if next_idx == 0 {
            String::new()
        } else {
            next_idx.to_string()
        };
        let new_stem = format!("{orig_stem}_Copy{suffix_num}");
        let new_name = match &ext {
            Some(e) if !e.is_empty() => format!("{new_stem}.{e}"),
            _ => new_stem,
        };
        current = current.with_file_name(new_name);
    }
    current
}

/// Outcome of a single file move during a merge.
#[derive(Debug, Clone)]
pub struct MergeMove {
    /// Source path.
    pub from: PathBuf,
    /// Final target path (after any `_Copy` suffix).
    pub to: PathBuf,
}

/// Report from [`merge_directories`].
#[derive(Debug, Clone, Default)]
pub struct MergeReport {
    /// Files moved (or that would be moved in dry-run).
    pub moved: Vec<MergeMove>,
}

fn walk_files(root: &Path) -> Result<Vec<PathBuf>, FrenError> {
    let mut out = Vec::new();
    walk_inner(root, &mut out)?;
    out.sort();
    Ok(out)
}

fn walk_inner(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), FrenError> {
    let entries = std::fs::read_dir(dir).map_err(|source| FrenError::Io {
        path: dir.to_path_buf(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| FrenError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        let p = entry.path();
        let ft = entry.file_type().map_err(|source| FrenError::Io {
            path: p.clone(),
            source,
        })?;
        if ft.is_dir() {
            walk_inner(&p, out)?;
        } else if ft.is_file() {
            out.push(p);
        }
        // symlinks: skip silently; merge isn't designed to handle them
    }
    Ok(())
}

fn unique_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // Mirrors Python: r"(?P<original_stem>.+)_copy(?P<index>\d+)?" with IGNORECASE.
        #[allow(clippy::expect_used)]
        Regex::new(r"(?i)^(?P<orig>.+)_copy(?P<idx>\d+)?$").expect("static unique regex compiles")
    })
}

/// Find a unique file name by appending `_Copy`, `_Copy1`, `_Copy2`, ... to
/// the stem until the path no longer exists.
pub fn unique_file_name(path: &Path) -> PathBuf {
    let mut current = path.to_path_buf();
    while current.exists() {
        let stem = current
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let ext = current
            .extension()
            .map(|e| e.to_string_lossy().into_owned());

        let (orig_stem, next_idx) = match unique_re().captures(&stem) {
            Some(caps) => {
                #[allow(clippy::expect_used)]
                let orig = caps
                    .name("orig")
                    .expect("regex named group orig")
                    .as_str()
                    .to_string();
                let idx_next = caps
                    .name("idx")
                    .and_then(|m| m.as_str().parse::<u32>().ok())
                    .map_or(1, |n| n + 1);
                (orig, idx_next)
            }
            None => (stem, 0),
        };

        let suffix_num = if next_idx == 0 {
            String::new()
        } else {
            next_idx.to_string()
        };
        let new_stem = format!("{orig_stem}_Copy{suffix_num}");
        let new_name = match &ext {
            Some(e) if !e.is_empty() => format!("{new_stem}.{e}"),
            _ => new_stem,
        };
        current = current.with_file_name(new_name);
    }
    current
}
