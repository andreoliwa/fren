#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Integration tests for the executor.

use fren_date::{execute, plan_with_year, ConflictPolicy, PlanOpts, SlugOpts};
use slug_preserve::CaseMode;
use std::fs;
use tempfile::TempDir;

fn touch(path: impl AsRef<std::path::Path>) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, b"x").unwrap();
}

fn rust_default_opts() -> SlugOpts {
    SlugOpts {
        separator: '-',
        case: CaseMode::Preserve,
        split_camel: false,
    }
}

#[test]
fn renames_files_with_dates() {
    let tmp = TempDir::new().unwrap();
    touch(tmp.path().join("Photo 2024-01-15.jpg"));

    let plans = plan_with_year(
        &[tmp.path()],
        &rust_default_opts(),
        &PlanOpts {
            recursive: true,
            ..PlanOpts::default()
        },
        2024,
    )
    .unwrap();

    let report = execute(&plans).unwrap();
    assert_eq!(report.applied, plans.len(), "all plans should apply");
    assert_eq!(report.errors.len(), 0);

    // The file should now have the slugified name on disk.
    assert!(tmp.path().join("Photo-2024-01-15.jpg").exists());
    assert!(!tmp.path().join("Photo 2024-01-15.jpg").exists());
}

#[test]
fn bottom_up_renames_dir_after_its_files() {
    let tmp = TempDir::new().unwrap();
    touch(tmp.path().join("My Vacation").join("photo 1.jpg"));
    touch(tmp.path().join("My Vacation").join("photo 2.jpg"));

    let plans = plan_with_year(
        &[tmp.path()],
        &rust_default_opts(),
        &PlanOpts {
            recursive: true,
            ..PlanOpts::default()
        },
        2024,
    )
    .unwrap();

    let report = execute(&plans).unwrap();
    assert_eq!(report.applied, plans.len());

    // Renamed dir + renamed children inside.
    let new_dir = tmp.path().join("My-Vacation");
    assert!(new_dir.exists(), "renamed dir must exist");
    assert!(new_dir.join("photo-1.jpg").exists());
    assert!(new_dir.join("photo-2.jpg").exists());
}

#[test]
fn aborts_when_target_exists_outside_batch() {
    let tmp = TempDir::new().unwrap();
    touch(tmp.path().join("Hello World.txt"));
    touch(tmp.path().join("Hello-World.txt")); // pre-existing target

    let result = plan_with_year(
        &[tmp.path()],
        &rust_default_opts(),
        &PlanOpts {
            recursive: true,
            on_conflict: ConflictPolicy::Abort,
            ..PlanOpts::default()
        },
        2024,
    );

    assert!(matches!(result, Err(fren_date::FrenError::TargetExists(_))));
    // Both files should still exist (no I/O happened).
    assert!(tmp.path().join("Hello World.txt").exists());
    assert!(tmp.path().join("Hello-World.txt").exists());
}

#[test]
fn does_not_overwrite_existing_file_on_execute() {
    // Even if a target somehow appears between planning and execution
    // (TOCTOU window), atomic rename refuses to overwrite.
    let tmp = TempDir::new().unwrap();
    touch(tmp.path().join("source.txt"));
    let plans = vec![fren_date::RenamePlan {
        original_path: tmp.path().join("source.txt"),
        parent: tmp.path().to_path_buf(),
        old_name: "source.txt".into(),
        new_name: "target.txt".into(),
        depth: 1,
        kind: fren_date::ItemKind::File,
        detected_date: None,
        batch_id: uuid::Uuid::nil(),
    }];

    // Now create the target after planning, before execution.
    touch(tmp.path().join("target.txt"));
    fs::write(tmp.path().join("target.txt"), b"PRESERVE_ME").unwrap();

    let report = execute(&plans).unwrap();
    assert_eq!(report.applied, 0, "execute should refuse to overwrite");
    assert_eq!(report.errors.len(), 1);

    // Target content preserved.
    let content = fs::read(tmp.path().join("target.txt")).unwrap();
    assert_eq!(content, b"PRESERVE_ME");
}

#[test]
fn empty_plans_succeed_with_zero_applied() {
    let report = execute(&[]).unwrap();
    assert_eq!(report.applied, 0);
    assert_eq!(report.errors.len(), 0);
}

#[test]
#[cfg(unix)]
fn symlink_renamed_not_target() {
    // Symlinks must be renamed as opaque filesystem entries - the link
    // itself moves, the target file is never touched or followed.
    let tmp = TempDir::new().unwrap();

    // Target lives outside the scanned tree so fren never touches it.
    let target_dir = TempDir::new().unwrap();
    touch(target_dir.path().join("real-file.txt"));

    // Symlink with a name fren will slugify; points outside the batch.
    std::os::unix::fs::symlink(
        target_dir.path().join("real-file.txt"),
        tmp.path().join("My Link.txt"),
    )
    .unwrap();

    let plans = plan_with_year(
        &[tmp.path()],
        &rust_default_opts(),
        &PlanOpts {
            recursive: true,
            ..PlanOpts::default()
        },
        2024,
    )
    .unwrap();

    // Only the symlink is in-scope; target is in a different tree.
    assert_eq!(plans.len(), 1, "expected exactly one plan (the symlink)");

    let plan = &plans[0];
    assert_eq!(plan.kind, fren_date::ItemKind::Symlink);
    assert_eq!(plan.old_name, std::ffi::OsStr::new("My Link.txt"));
    assert_eq!(plan.new_name, std::ffi::OsStr::new("My-Link.txt"));

    let report = execute(&plans).unwrap();
    assert_eq!(report.applied, 1);
    assert_eq!(report.errors.len(), 0);

    // Renamed symlink is still a symlink and still resolves to the target.
    let new_link = tmp.path().join("My-Link.txt");
    assert!(
        new_link
            .symlink_metadata()
            .unwrap()
            .file_type()
            .is_symlink(),
        "renamed entry must still be a symlink"
    );
    assert!(new_link.exists(), "symlink must still resolve after rename");

    // Old symlink name gone; target file untouched.
    assert!(!tmp.path().join("My Link.txt").exists());
    assert!(target_dir.path().join("real-file.txt").exists());
}
