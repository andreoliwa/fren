//! Conflict detection for the planner.
//!
//! Two distinct conflict classes:
//!
//! 1. **Within-batch:** two plans want to rename to the same target. Always
//!    aborts (different files cannot share a target).
//! 2. **Pre-existing target:** the target path already exists on disk and
//!    is not part of the batch. Aborts under `ConflictPolicy::Abort` (the
//!    only currently-supported policy).

use crate::{FrenError, RenamePlan};
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

/// Verify no two plans target the same final path.
pub fn check_within_batch(plans: &[RenamePlan]) -> Result<(), FrenError> {
    let mut targets: HashMap<PathBuf, &RenamePlan> = HashMap::new();
    for plan in plans {
        let target = plan.parent.join(&plan.new_name);
        if let Some(prev) = targets.insert(target.clone(), plan) {
            return Err(FrenError::WithinBatchCollision {
                a: prev.original_path.clone(),
                b: plan.original_path.clone(),
                target,
            });
        }
    }
    Ok(())
}

/// Verify no plan's target already exists on disk outside the batch.
///
/// Plans whose target equals another plan's source are exempt - that is the
/// chain case (`a -> b`, `b -> c`) which the bottom-up executor handles
/// safely as long as ordering is correct.
pub fn check_preexisting(plans: &[RenamePlan]) -> Result<(), FrenError> {
    let plan_sources: std::collections::HashSet<&PathBuf> =
        plans.iter().map(|p| &p.original_path).collect();
    for plan in plans {
        let target = plan.parent.join(&plan.new_name);
        if plan_sources.contains(&target) {
            // Some other plan will move this path away first.
            continue;
        }
        if target.exists() {
            return Err(FrenError::TargetExists(target));
        }
    }
    Ok(())
}

/// Build a `{stem}-copy-{n}{ext}` variant of `base_name`.
///
/// Uses the same stem/extension split as `std::path::Path::file_stem()` and
/// `Path::extension()` - only the final extension is separated. Examples:
///
/// - `"report.tar.gz"` with n=2 gives `"report.tar-copy-2.gz"`
/// - `"README"` with n=1 gives `"README-copy-1"` (no extension)
/// - `".gitignore"` with n=1 gives `".gitignore-copy-1"` (leading-dot dotfile
///   has an empty extension per `std::path` semantics)
// Will be called by resolve_within_batch and resolve_preexisting once those are added.
#[allow(dead_code)]
fn numbered_name(base_name: &OsStr, n: u32) -> OsString {
    let p = std::path::Path::new(base_name);
    let stem = p
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ext = p
        .extension()
        .map(|e| e.to_string_lossy().into_owned())
        .unwrap_or_default();
    if ext.is_empty() {
        format!("{stem}-copy-{n}").into()
    } else {
        format!("{stem}-copy-{n}.{ext}").into()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use std::ffi::OsString;

    #[test]
    fn numbered_name_with_extension() {
        let result = numbered_name(OsStr::new("2022-02-21t18-59-15.mp3"), 1);
        assert_eq!(result, OsString::from("2022-02-21t18-59-15-copy-1.mp3"));
    }

    #[test]
    fn numbered_name_multi_dot_keeps_only_final_ext() {
        let result = numbered_name(OsStr::new("report.tar.gz"), 2);
        assert_eq!(result, OsString::from("report.tar-copy-2.gz"));
    }

    #[test]
    fn numbered_name_no_extension() {
        let result = numbered_name(OsStr::new("README"), 1);
        assert_eq!(result, OsString::from("README-copy-1"));
    }

    #[test]
    fn numbered_name_dotfile_empty_ext() {
        // Leading-dot dotfiles: std::path treats the whole name as the stem,
        // with no extension.
        let result = numbered_name(OsStr::new(".gitignore"), 1);
        assert_eq!(result, OsString::from(".gitignore-copy-1"));
    }

    #[test]
    fn numbered_name_single_char_ext() {
        let result = numbered_name(OsStr::new("a.b"), 3);
        assert_eq!(result, OsString::from("a-copy-3.b"));
    }
}
