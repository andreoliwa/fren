//! Conflict detection and resolution for the planner.
//!
//! Two distinct conflict classes:
//!
//! 1. **Within-batch:** two plans want to rename to the same target.
//! 2. **Pre-existing target:** the target path already exists on disk and
//!    is not part of the batch.
//!
//! Both classes are handled according to the active `ConflictPolicy`:
//!
//! - `Abort`: return an error immediately (original behaviour).
//! - `Number`: rename the losing plan to a `{stem}-copy-{n}{ext}` variant.
//! - `Skip`/`Merge`: reserved, return `NotYetImplemented`.

use crate::{ConflictPolicy, FrenError, RenamePlan};
use std::collections::{HashMap, HashSet};
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

/// Resolve within-batch conflicts according to `policy`.
///
/// Under `Abort`: returns `Err(FrenError::WithinBatchCollision)` on the first
/// collision.
///
/// Under `Number`: groups plans by computed target. Within each colliding
/// group, the plan whose original basename sorts first lexicographically keeps
/// the plain target; every other plan is renamed to a `{stem}-copy-{k}`
/// variant (k starting at 1, ascending). The `plans` vector is mutated
/// in place.
pub fn resolve_within_batch(
    plans: &mut [RenamePlan],
    policy: ConflictPolicy,
) -> Result<(), FrenError> {
    // Group plan indices by target path.
    let target_map: HashMap<PathBuf, Vec<usize>> = {
        let mut m: HashMap<PathBuf, Vec<usize>> = HashMap::new();
        for (i, plan) in plans.iter().enumerate() {
            let target = plan.parent.join(&plan.new_name);
            m.entry(target).or_default().push(i);
        }
        m
    };

    // Collect all initially-claimed target paths into a separate set so that
    // we can track newly-assigned copy names without mutating `target_map`
    // while iterating over it (borrow-checker constraint).
    let mut claimed: HashSet<PathBuf> = target_map.keys().cloned().collect();

    // Collect colliding groups (>1 member) up front so we don't borrow
    // `target_map` across the mutable `claimed` updates below.
    let colliding: Vec<(PathBuf, Vec<usize>)> = target_map
        .iter()
        .filter(|(_, indices)| indices.len() > 1)
        .map(|(t, indices)| (t.clone(), indices.clone()))
        .collect();

    for (target, indices) in colliding {
        match policy {
            ConflictPolicy::Abort => {
                let a = plans[indices[0]].original_path.clone();
                let b = plans[indices[1]].original_path.clone();
                return Err(FrenError::WithinBatchCollision {
                    a,
                    b,
                    target: target.clone(),
                });
            }
            ConflictPolicy::Number => {
                // Sort group members by their original basename lexicographically.
                let mut sorted: Vec<usize> = indices.clone();
                sorted.sort_by_key(|&i| {
                    plans[i]
                        .original_path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default()
                });
                // Index 0 keeps the plain target; others get -copy-{k}.
                // Extract the plain base name from the target.
                let plain_base: OsString =
                    target.file_name().map(OsString::from).unwrap_or_default();
                let parent = plans[sorted[0]].parent.clone();
                let mut k = 1u32;
                for &plan_idx in sorted.iter().skip(1) {
                    // Scan for a free slot using the global `claimed` set.
                    // This prevents cross-group collisions: if another group's
                    // plain target is already "foo-copy-1", we skip k=1 and
                    // try k=2, rather than assigning a duplicate.
                    loop {
                        let candidate = numbered_name(&plain_base, k);
                        let candidate_path = parent.join(&candidate);
                        k += 1;
                        if !claimed.contains(&candidate_path) {
                            plans[plan_idx].new_name = candidate;
                            // Mark as claimed so subsequent groups avoid it.
                            claimed.insert(candidate_path);
                            break;
                        }
                    }
                }
            }
            ConflictPolicy::Skip | ConflictPolicy::Merge => {
                return Err(FrenError::NotYetImplemented(
                    "Skip/Merge conflict policy".to_string(),
                ));
            }
        }
    }
    Ok(())
}

/// Resolve pre-existing-target conflicts according to `policy`.
///
/// Plans whose target equals another plan's source are exempt (the chain
/// case: `a -> b`, `b -> c`). The bottom-up executor handles chain ordering
/// safely.
///
/// Under `Abort`: returns `Err(FrenError::TargetExists)` on the first
/// collision.
///
/// Under `Number`: for each plan whose target already exists on disk (and is
/// not chain-exempt), increments `n` from 1 until a `{stem}-copy-{n}` variant
/// is free both on disk and in the in-batch target set, then mutates
/// `plan.new_name` to that variant. A defensive cap of 10 000 iterations
/// prevents an infinite loop (returns `NotYetImplemented` if exceeded).
pub fn resolve_preexisting(
    plans: &mut [RenamePlan],
    policy: ConflictPolicy,
) -> Result<(), FrenError> {
    // Collect owned copies so we can iterate plans mutably below.
    let plan_sources: HashSet<PathBuf> = plans.iter().map(|p| p.original_path.clone()).collect();

    // Track all current in-batch targets so the numbering loop avoids them.
    let mut batch_targets: HashSet<PathBuf> =
        plans.iter().map(|p| p.parent.join(&p.new_name)).collect();

    for plan in plans.iter_mut() {
        let target = plan.parent.join(&plan.new_name);
        if plan_sources.contains(&target) {
            // Chain case: another plan will vacate this path first.
            continue;
        }
        if !target.exists() {
            continue;
        }
        match policy {
            ConflictPolicy::Abort => {
                return Err(FrenError::TargetExists(target));
            }
            ConflictPolicy::Number => {
                // Remove the current (colliding) target from the batch set so
                // we can reassign without false-conflicting with ourselves.
                batch_targets.remove(&target);
                // Strip any existing -copy-N suffix so we always number from
                // the original root name (e.g. "i-copy-1" -> base "i",
                // yielding "i-copy-2" rather than "i-copy-1-copy-1").
                let base_name = strip_copy_suffix(&plan.new_name);
                let mut found = false;
                for n in 1u32..=10_000 {
                    let candidate_name = numbered_name(&base_name, n);
                    let candidate_path = plan.parent.join(&candidate_name);
                    if !candidate_path.exists() && !batch_targets.contains(&candidate_path) {
                        plan.new_name = candidate_name;
                        batch_targets.insert(candidate_path);
                        found = true;
                        break;
                    }
                }
                if !found {
                    return Err(FrenError::NotYetImplemented(
                        "conflict numbering exceeded 10000 attempts".to_string(),
                    ));
                }
            }
            ConflictPolicy::Skip | ConflictPolicy::Merge => {
                return Err(FrenError::NotYetImplemented(
                    "Skip/Merge conflict policy".to_string(),
                ));
            }
        }
    }
    Ok(())
}

/// Return the root (pre-copy) base name for `name`, stripping any trailing
/// `-copy-{n}` suffix that was added by a previous numbering pass.
///
/// This is used by `resolve_preexisting` so that a name like `"i-copy-1"`
/// generated by `resolve_within_batch` is treated as root `"i"` when
/// searching for the next free slot, yielding `"i-copy-2"` rather than
/// `"i-copy-1-copy-1"`.
///
/// The extension (if any) is preserved. Only an exact `-copy-{digits}` suffix
/// on the stem is stripped; other names pass through unchanged.
fn strip_copy_suffix(name: &OsStr) -> OsString {
    let p = std::path::Path::new(name);
    let stem = p
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ext = p
        .extension()
        .map(|e| e.to_string_lossy().into_owned())
        .unwrap_or_default();
    // Strip `-copy-<digits>` from the end of the stem, if present.
    let root_stem = if let Some(pos) = stem.rfind("-copy-") {
        let suffix = &stem[pos + 6..]; // after "-copy-"
        if !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()) {
            &stem[..pos]
        } else {
            &stem
        }
    } else {
        &stem
    };
    if ext.is_empty() {
        root_stem.to_string().into()
    } else {
        format!("{root_stem}.{ext}").into()
    }
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
    use crate::{ItemKind, RenamePlan};
    use std::ffi::{OsStr, OsString};
    use tempfile::TempDir;
    use uuid::Uuid;

    // -----------------------------------------------------------------------
    // numbered_name tests
    // -----------------------------------------------------------------------

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

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn make_plan(dir: &std::path::Path, original: &str, new: &str) -> RenamePlan {
        RenamePlan {
            original_path: dir.join(original),
            parent: dir.to_path_buf(),
            old_name: OsString::from(original),
            new_name: OsString::from(new),
            depth: 0,
            kind: ItemKind::File,
            detected_date: None,
            batch_id: Uuid::nil(),
        }
    }

    // -----------------------------------------------------------------------
    // resolve_within_batch tests
    // -----------------------------------------------------------------------

    #[test]
    fn within_batch_abort_returns_collision_error() {
        let tmp = TempDir::new().unwrap();
        let mut plans = vec![
            make_plan(tmp.path(), "i", "i"),
            make_plan(tmp.path(), "I", "i"),
        ];
        let result = resolve_within_batch(&mut plans, ConflictPolicy::Abort);
        assert!(
            matches!(result, Err(FrenError::WithinBatchCollision { .. })),
            "expected WithinBatchCollision, got {result:?}"
        );
    }

    #[test]
    fn within_batch_number_two_sources_alpha_first_keeps_plain() {
        let tmp = TempDir::new().unwrap();
        // "I" sorts before "i" ASCII-wise (uppercase < lowercase).
        let mut plans = vec![
            make_plan(tmp.path(), "i", "i"),
            make_plan(tmp.path(), "I", "i"),
        ];
        resolve_within_batch(&mut plans, ConflictPolicy::Number).unwrap();
        // Find which plan has original "I" and which has "i".
        let plan_big_i = plans
            .iter()
            .find(|p| p.old_name == OsStr::new("I"))
            .unwrap();
        let plan_small_i = plans
            .iter()
            .find(|p| p.old_name == OsStr::new("i"))
            .unwrap();
        // "I" (sorts first lexicographically) keeps the plain target.
        assert_eq!(
            plan_big_i.new_name,
            OsStr::new("i"),
            "alpha-first plan should keep plain target"
        );
        // "i" (sorts second) gets -copy-1.
        assert_eq!(
            plan_small_i.new_name,
            OsStr::new("i-copy-1"),
            "alpha-second plan should get -copy-1"
        );
    }

    #[test]
    fn within_batch_number_three_sources_get_sequential_copies() {
        let tmp = TempDir::new().unwrap();
        // "I" < "I " < "i" in lexicographic order.
        let mut plans = vec![
            make_plan(tmp.path(), "i", "i"),
            make_plan(tmp.path(), "I", "i"),
            make_plan(tmp.path(), "I ", "i"),
        ];
        resolve_within_batch(&mut plans, ConflictPolicy::Number).unwrap();
        let new_names: std::collections::BTreeMap<String, String> = plans
            .iter()
            .map(|p| {
                (
                    p.old_name.to_string_lossy().to_string(),
                    p.new_name.to_string_lossy().to_string(),
                )
            })
            .collect();
        assert_eq!(new_names["I"], "i");
        assert_eq!(new_names["I "], "i-copy-1");
        assert_eq!(new_names["i"], "i-copy-2");
    }

    #[test]
    fn within_batch_number_cross_group_collision_skips_taken_slot() {
        // CR-01 regression: group A has {src0, src1} both targeting "foo";
        // group B has {src2} targeting "foo-copy-1" (a different source).
        // Old code: src1 gets "foo-copy-1" (k=1), colliding with src2.
        // Fixed code: src1 skips "foo-copy-1" (taken by group B) and gets
        // "foo-copy-2" instead.
        let tmp = TempDir::new().unwrap();
        let mut plans = vec![
            make_plan(tmp.path(), "src0", "foo"),
            make_plan(tmp.path(), "src1", "foo"),
            make_plan(tmp.path(), "src2", "foo-copy-1"),
        ];
        resolve_within_batch(&mut plans, ConflictPolicy::Number).unwrap();
        let names: std::collections::BTreeMap<String, String> = plans
            .iter()
            .map(|p| {
                (
                    p.old_name.to_string_lossy().to_string(),
                    p.new_name.to_string_lossy().to_string(),
                )
            })
            .collect();
        // src0 keeps "foo" (alpha-first).
        assert_eq!(names["src0"], "foo");
        // src2 keeps "foo-copy-1" (single-member group, untouched).
        assert_eq!(names["src2"], "foo-copy-1");
        // src1 must NOT collide with src2's target; it should get "foo-copy-2".
        assert_eq!(
            names["src1"], "foo-copy-2",
            "cross-group collision: src1 should skip foo-copy-1 (taken by src2)"
        );
    }

    #[test]
    fn within_batch_no_conflict_is_unchanged() {
        let tmp = TempDir::new().unwrap();
        let mut plans = vec![
            make_plan(tmp.path(), "a", "alpha"),
            make_plan(tmp.path(), "b", "beta"),
        ];
        resolve_within_batch(&mut plans, ConflictPolicy::Number).unwrap();
        assert_eq!(plans[0].new_name, OsString::from("alpha"));
        assert_eq!(plans[1].new_name, OsString::from("beta"));
    }

    // -----------------------------------------------------------------------
    // resolve_preexisting tests
    // -----------------------------------------------------------------------

    #[test]
    fn preexisting_abort_returns_target_exists_error() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("i"), b"").unwrap();
        let mut plans = vec![make_plan(tmp.path(), "j", "i")];
        let result = resolve_preexisting(&mut plans, ConflictPolicy::Abort);
        assert!(
            matches!(result, Err(FrenError::TargetExists(_))),
            "expected TargetExists, got {result:?}"
        );
    }

    #[test]
    fn preexisting_number_renames_to_copy_1() {
        let tmp = TempDir::new().unwrap();
        // Create pre-existing target "i" on disk.
        std::fs::write(tmp.path().join("i"), b"").unwrap();
        let mut plans = vec![make_plan(tmp.path(), "j", "i")];
        resolve_preexisting(&mut plans, ConflictPolicy::Number).unwrap();
        assert_eq!(
            plans[0].new_name,
            OsString::from("i-copy-1"),
            "should rename to i-copy-1 when i exists"
        );
    }

    #[test]
    fn preexisting_number_skips_over_occupied_copies() {
        let tmp = TempDir::new().unwrap();
        // "i" and "i-copy-1" both exist; plan should resolve to "i-copy-2".
        std::fs::write(tmp.path().join("i"), b"").unwrap();
        std::fs::write(tmp.path().join("i-copy-1"), b"").unwrap();
        let mut plans = vec![make_plan(tmp.path(), "j", "i")];
        resolve_preexisting(&mut plans, ConflictPolicy::Number).unwrap();
        assert_eq!(plans[0].new_name, OsString::from("i-copy-2"));
    }

    #[test]
    fn preexisting_chain_exemption_preserved() {
        let tmp = TempDir::new().unwrap();
        // plan A: rename "a" -> "b"; plan B: rename "b" -> "c".
        // "b" already exists on disk, but plan B's target ("c") is fine.
        // plan A's target "b" should be exempt because it equals plan B's source.
        let plan_a = make_plan(tmp.path(), "a", "b");
        let plan_b = make_plan(tmp.path(), "b", "c");
        std::fs::write(tmp.path().join("b"), b"").unwrap();
        let mut plans = vec![plan_a, plan_b];
        // Chain exemption: plan_a targets "b", which is plan_b's source.
        // Should not error.
        resolve_preexisting(&mut plans, ConflictPolicy::Abort).unwrap();
        // Verify plans are unmodified.
        assert_eq!(plans[0].new_name, OsString::from("b"));
        assert_eq!(plans[1].new_name, OsString::from("c"));
    }

    #[test]
    fn preexisting_no_conflict_unchanged() {
        let tmp = TempDir::new().unwrap();
        let mut plans = vec![make_plan(tmp.path(), "old", "new")];
        // "new" does not exist on disk.
        resolve_preexisting(&mut plans, ConflictPolicy::Number).unwrap();
        assert_eq!(plans[0].new_name, OsString::from("new"));
    }

    #[test]
    fn within_batch_then_preexisting_combined_scenario() {
        // "i" and "I" both map to "i"; "i-copy-1" already exists on disk.
        // After within-batch: "I" -> "i", "i" -> "i-copy-1".
        // After preexisting: "i-copy-1" already on disk, so "i" -> "i-copy-2".
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("i-copy-1"), b"").unwrap();
        let mut plans = vec![
            make_plan(tmp.path(), "i", "i"),
            make_plan(tmp.path(), "I", "i"),
        ];
        resolve_within_batch(&mut plans, ConflictPolicy::Number).unwrap();
        resolve_preexisting(&mut plans, ConflictPolicy::Number).unwrap();

        let plan_big_i = plans
            .iter()
            .find(|p| p.old_name == OsStr::new("I"))
            .unwrap();
        let plan_small_i = plans
            .iter()
            .find(|p| p.old_name == OsStr::new("i"))
            .unwrap();
        assert_eq!(plan_big_i.new_name, OsStr::new("i"));
        assert_eq!(plan_small_i.new_name, OsStr::new("i-copy-2"));
    }
}
