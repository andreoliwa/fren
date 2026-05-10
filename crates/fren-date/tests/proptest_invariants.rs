#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Property-based tests for executor invariants (§8.3).
//!
//! Invariant 1: No file outside the batch is modified.
//! Invariant 2: Post-state is one of:
//!   - all-applied  (every planned rename completed)
//!   - clean-abort  (zero renames happened, TargetExists returned)
//!   - partial-with-complete-log (not yet tested; requires tx log feature)

use fren_date::{execute, plan_with_year, ConflictPolicy, FrenError, PlanOpts, SlugOpts};
use proptest::prelude::*;
use slug_preserve::CaseMode;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn default_opts() -> SlugOpts {
    SlugOpts {
        separator: '-',
        case: CaseMode::Preserve,
        split_camel: false,
    }
}

fn default_plan_opts() -> PlanOpts {
    PlanOpts {
        recursive: true,
        on_conflict: ConflictPolicy::Abort,
        ..PlanOpts::default()
    }
}

/// Create a file (and parent dirs) at `path`, writing `b"x"`.
fn touch(path: &Path) {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).unwrap();
    }
    fs::write(path, b"x").unwrap();
}

/// Snapshot of a directory: maps relative path → file size.
/// Used to detect any mutation outside the batch dir.
fn snapshot(dir: &Path) -> BTreeMap<PathBuf, u64> {
    let mut map = BTreeMap::new();
    for entry in walkdir::WalkDir::new(dir).min_depth(1) {
        let entry = entry.unwrap();
        let meta = entry.metadata().unwrap();
        if meta.is_file() {
            let rel = entry.path().strip_prefix(dir).unwrap().to_path_buf();
            map.insert(rel, meta.len());
        }
    }
    map
}

/// A valid filename component: non-empty, no `/`, no null byte, not `.` or `..`,
/// max 60 chars to stay well within typical FS limits.
fn filename_strategy() -> impl Strategy<Value = String> {
    "[A-Za-z0-9 _.,()-]{1,30}".prop_filter("not . or ..", |s| s != "." && s != "..")
}

// ---------------------------------------------------------------------------
// Invariant 1: nothing outside the batch dir is touched
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn invariant1_nothing_outside_batch_modified(
        filenames in prop::collection::vec(filename_strategy(), 1..8),
    ) {
        let batch_dir = TempDir::new().unwrap();
        let sentinel_dir = TempDir::new().unwrap();

        // Place a sentinel file in a sibling dir (outside the batch).
        let sentinel = sentinel_dir.path().join("sentinel.txt");
        fs::write(&sentinel, b"SENTINEL").unwrap();

        // Snapshot the sentinel dir before execution.
        let before = snapshot(sentinel_dir.path());

        // Create the batch files.
        let mut created = std::collections::HashSet::new();
        for name in &filenames {
            if created.insert(name.clone()) {
                touch(&batch_dir.path().join(name));
            }
        }

        // Plan + execute (ignore errors; we only care about the invariant).
        let plans = plan_with_year(
            &[batch_dir.path()],
            &default_opts(),
            &default_plan_opts(),
            2024,
        );
        if let Ok(plans) = plans {
            let _ = execute(&plans);
        }

        // Sentinel dir must be byte-for-byte identical.
        let after = snapshot(sentinel_dir.path());
        prop_assert_eq!(before, after, "sentinel dir mutated — batch leaked outside");

        // The sentinel file content must be unchanged.
        let content = fs::read(&sentinel).unwrap();
        prop_assert_eq!(content, b"SENTINEL".to_vec(), "sentinel file content changed");
    }
}

// ---------------------------------------------------------------------------
// Invariant 2: post-state is all-applied or clean-abort
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn invariant2_post_state_is_consistent(
        filenames in prop::collection::vec(filename_strategy(), 1..8),
    ) {
        let batch_dir = TempDir::new().unwrap();

        // Deduplicate so each name is unique on disk.
        let mut unique: Vec<String> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for n in filenames {
            if seen.insert(n.clone()) {
                unique.push(n);
            }
        }

        for name in &unique {
            touch(&batch_dir.path().join(name));
        }

        let files_before: usize = unique.len();

        let plans = plan_with_year(
            &[batch_dir.path()],
            &default_opts(),
            &default_plan_opts(),
            2024,
        );

        match plans {
            Err(FrenError::TargetExists(_)) => {
                // Clean abort: planner detected a pre-existing target.
                // No renames should have happened (planner aborts before execute).
                let files_after = count_files(batch_dir.path());
                prop_assert_eq!(
                    files_after, files_before,
                    "clean-abort: file count changed"
                );
            }
            Err(FrenError::WithinBatchCollision { .. }) => {
                // Two source names would map to the same target: planner aborts.
                let files_after = count_files(batch_dir.path());
                prop_assert_eq!(
                    files_after, files_before,
                    "within-batch-collision abort: file count changed"
                );
            }
            Err(_) => {
                // Other planner errors (I/O etc.) - still no renames happened.
            }
            Ok(plans) => {
                let planned = plans.len();
                let report = execute(&plans).unwrap();

                if report.errors.is_empty() {
                    // All-applied: every planned rename completed.
                    prop_assert_eq!(
                        report.applied, planned,
                        "all-applied: applied count mismatch"
                    );
                } else {
                    // Partial or clean-abort-at-execute (TargetExists during execute):
                    // applied + errors should account for all plans attempted.
                    // We can't verify the tx log yet (not implemented), so we only
                    // check that applied ≤ planned and errors ≥ 1.
                    prop_assert!(report.applied <= planned);
                    prop_assert!(!report.errors.is_empty());
                }

                // File count must be conserved: renames don't create or destroy files.
                let files_after = count_files(batch_dir.path());
                prop_assert_eq!(
                    files_after, files_before,
                    "file count changed after execute (files created or lost)"
                );
            }
        }
    }
}

fn count_files(dir: &Path) -> usize {
    walkdir::WalkDir::new(dir)
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .count()
}
