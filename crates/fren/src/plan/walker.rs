//! Filesystem traversal for planning.
//!
//! Walks a root directory and returns every visible item with its depth.
//! Hidden entries (names starting with `.`) are always skipped.
//!
//! Symlinks are returned as `ItemKind::Symlink` and never followed.

use crate::{plan_types::ItemKind, FrenError, PlanOpts};
use std::path::{Path, PathBuf};

/// One filesystem entry discovered during a walk.
#[derive(Debug, Clone)]
pub(crate) struct DiscoveredItem {
    /// Absolute (or root-relative) path of the entry.
    pub path: PathBuf,
    /// What kind of entry this is.
    pub kind: ItemKind,
    /// Depth from the walk root (root itself = 0).
    pub depth: usize,
}

/// Walk `root` and return every visible entry (excluding hidden files/dirs
/// and any path under `opts.exclude`). Includes the root itself.
pub fn walk(root: &Path, opts: &PlanOpts) -> Result<Vec<DiscoveredItem>, FrenError> {
    let mut out = Vec::new();
    walk_inner(root, root, 0, opts, &mut out)?;
    Ok(out)
}

fn walk_inner(
    root: &Path,
    current: &Path,
    depth: usize,
    opts: &PlanOpts,
    out: &mut Vec<DiscoveredItem>,
) -> Result<(), FrenError> {
    let metadata = std::fs::symlink_metadata(current).map_err(|source| FrenError::Io {
        path: current.to_path_buf(),
        source,
    })?;

    let kind = if metadata.file_type().is_symlink() {
        ItemKind::Symlink
    } else if metadata.is_dir() {
        ItemKind::Dir
    } else {
        ItemKind::File
    };

    out.push(DiscoveredItem {
        path: current.to_path_buf(),
        kind,
        depth,
    });

    // Don't recurse into symlinks. Don't recurse into non-dirs.
    if kind != ItemKind::Dir {
        return Ok(());
    }

    // Don't recurse if the user asked for non-recursive and we're past
    // the root.
    if !opts.recursive && depth > 0 {
        return Ok(());
    }

    let entries = std::fs::read_dir(current).map_err(|source| FrenError::Io {
        path: current.to_path_buf(),
        source,
    })?;

    for entry in entries {
        let entry = entry.map_err(|source| FrenError::Io {
            path: current.to_path_buf(),
            source,
        })?;
        let child_path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Skip hidden entries.
        if name_str.starts_with('.') {
            continue;
        }

        // Skip excluded paths (any prefix match).
        if opts.exclude.iter().any(|excl| child_path.starts_with(excl)) {
            continue;
        }

        // Don't follow symlinks even if they point to dirs.
        let child_meta = entry.file_type().map_err(|source| FrenError::Io {
            path: child_path.clone(),
            source,
        })?;
        if child_meta.is_symlink() {
            out.push(DiscoveredItem {
                path: child_path,
                kind: ItemKind::Symlink,
                depth: depth + 1,
            });
            continue;
        }

        let _ = root; // reserved for future relative-path tracking
        walk_inner(root, &child_path, depth + 1, opts, out)?;
    }
    Ok(())
}
