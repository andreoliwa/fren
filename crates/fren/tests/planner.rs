#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Integration tests for the planner.

use fren::{plan_with_year, ConflictPolicy, ItemKind, PlanOpts, SlugOpts};
use slug_preserve::CaseMode;
use std::fs;
use tempfile::TempDir;

fn touch(path: impl AsRef<std::path::Path>) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap_or_else(|e| panic!("mkdir {}: {}", parent.display(), e));
    }
    fs::write(path, b"").unwrap_or_else(|e| panic!("write {}: {}", path.display(), e));
}

fn rust_default_opts() -> SlugOpts {
    SlugOpts {
        separator: '-',
        case: CaseMode::Preserve,
        split_camel: false,
    }
}

#[test]
fn walks_recursively_and_skips_hidden() {
    let tmp = TempDir::new().expect("tempdir");
    touch(tmp.path().join("Hello World.txt"));
    touch(tmp.path().join("subdir").join("Inner File.txt"));
    touch(tmp.path().join(".hidden").join("ignored.txt"));
    fs::write(tmp.path().join(".hidden_file"), b"").expect("write hidden file");

    let plans = plan_with_year(
        &[tmp.path()],
        &rust_default_opts(),
        &PlanOpts {
            recursive: true,
            ..PlanOpts::default()
        },
        2024,
    )
    .expect("plan ok");

    let originals: Vec<_> = plans
        .iter()
        .map(|p| {
            p.original_path
                .strip_prefix(tmp.path())
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .collect();

    // Hidden dir + file excluded; subdir + its contents present.
    assert!(originals.iter().any(|p| p == "Hello World.txt"));
    assert!(originals.iter().any(|p| p == "subdir/Inner File.txt"));
    // Subdir itself: only present if its renamed name differs from its
    // original. "subdir" → "subdir" (no spaces, no dates) is a no-op so
    // it's filtered out by plan().
    assert!(!originals.iter().any(|p| p.starts_with(".hidden")));
}

#[test]
fn bottom_up_sort_files_then_dirs() {
    let tmp = TempDir::new().expect("tempdir");
    touch(tmp.path().join("My Vacation").join("photo 1.txt"));
    touch(tmp.path().join("My Vacation").join("photo 2.txt"));

    let plans = plan_with_year(
        &[tmp.path()],
        &rust_default_opts(),
        &PlanOpts {
            recursive: true,
            ..PlanOpts::default()
        },
        2024,
    )
    .expect("plan ok");

    // Files (depth 2) come before dir (depth 1).
    let mut prev_depth = usize::MAX;
    let mut saw_file_at_deepest = false;
    for plan in &plans {
        if plan.depth == 2 && matches!(plan.kind, ItemKind::File) {
            saw_file_at_deepest = true;
        }
        assert!(
            plan.depth <= prev_depth,
            "depths must monotonically decrease (deepest first), got {} after {}",
            plan.depth,
            prev_depth
        );
        prev_depth = plan.depth;
    }
    assert!(saw_file_at_deepest, "expected to find a file at depth 2");
}

#[test]
fn no_op_renames_are_filtered() {
    let tmp = TempDir::new().expect("tempdir");
    // Already in desired form (preserve case, dash separator, no date).
    touch(tmp.path().join("already-good.txt"));

    let plans = plan_with_year(
        &[tmp.path()],
        &rust_default_opts(),
        &PlanOpts {
            recursive: true,
            ..PlanOpts::default()
        },
        2024,
    )
    .expect("plan ok");

    // The good file should NOT appear in the plan.
    let originals: Vec<_> = plans
        .iter()
        .map(|p| {
            p.original_path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert!(
        !originals.iter().any(|n| n == "already-good.txt"),
        "no-op should be filtered, got: {originals:?}"
    );
}

#[test]
fn within_batch_collision_detected() {
    let tmp = TempDir::new().expect("tempdir");
    // Both will slugify to the same name.
    touch(tmp.path().join("Hello World.txt"));
    touch(tmp.path().join("Hello   World.txt")); // multiple spaces

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

    match result {
        Err(fren::FrenError::WithinBatchCollision { .. }) => {}
        other => panic!("expected WithinBatchCollision, got {other:?}"),
    }
}

#[test]
fn preexisting_target_aborts() {
    let tmp = TempDir::new().expect("tempdir");
    touch(tmp.path().join("Hello World.txt"));
    // Target already exists.
    touch(tmp.path().join("Hello-World.txt"));

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

    match result {
        Err(fren::FrenError::TargetExists(_)) => {}
        other => panic!("expected TargetExists, got {other:?}"),
    }
}

#[test]
fn exclude_paths_are_skipped() {
    let tmp = TempDir::new().expect("tempdir");
    touch(tmp.path().join("Keep Me.txt"));
    touch(tmp.path().join("Skip Me").join("inside.txt"));

    let plans = plan_with_year(
        &[tmp.path()],
        &rust_default_opts(),
        &PlanOpts {
            recursive: true,
            exclude: vec![tmp.path().join("Skip Me")],
            ..PlanOpts::default()
        },
        2024,
    )
    .expect("plan ok");

    let originals: Vec<_> = plans
        .iter()
        .map(|p| p.original_path.to_string_lossy().into_owned())
        .collect();
    assert!(originals.iter().any(|p| p.contains("Keep Me.txt")));
    assert!(
        !originals.iter().any(|p| p.contains("Skip Me")),
        "excluded path leaked: {originals:?}"
    );
}

#[test]
fn extension_lowercased() {
    let tmp = TempDir::new().expect("tempdir");
    // "My Doc" + ".PDF" → after slugify "My-Doc" with lowercased ext.
    // Using a name with spaces so the new path differs from the old on
    // case-insensitive filesystems (macOS APFS-ci, Windows NTFS-ci).
    touch(tmp.path().join("My Doc.PDF"));

    let plans = plan_with_year(
        &[tmp.path()],
        &rust_default_opts(),
        &PlanOpts {
            recursive: true,
            ..PlanOpts::default()
        },
        2024,
    )
    .expect("plan ok");

    let new_names: Vec<String> = plans
        .iter()
        .map(|p| p.new_name.to_string_lossy().into_owned())
        .collect();
    assert!(
        new_names.iter().any(|n| n == "My-Doc.pdf"),
        "extension should be lowercased: {new_names:?}"
    );
}

#[test]
fn date_detected_in_filename() {
    let tmp = TempDir::new().expect("tempdir");
    touch(tmp.path().join("photo_20240115.jpg"));

    let plans = plan_with_year(
        &[tmp.path()],
        &rust_default_opts(),
        &PlanOpts {
            recursive: true,
            ..PlanOpts::default()
        },
        2024,
    )
    .expect("plan ok");

    let new_names: Vec<String> = plans
        .iter()
        .map(|p| p.new_name.to_string_lossy().into_owned())
        .collect();
    // With dash separator and Preserve case: "photo-2024-01-15.jpg"
    assert!(
        new_names.iter().any(|n| n == "photo-2024-01-15.jpg"),
        "expected date detection: {new_names:?}"
    );
}
