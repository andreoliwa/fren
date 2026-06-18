#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Integration tests for the reorder planner.

use fren_date::{plan_reorder_with_year, plan_with_year, PlanOpts, SlugOpts};
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
fn reorder_moves_date_to_front() {
    let tmp = TempDir::new().expect("tempdir");
    touch(tmp.path().join("IMG-01-12-2025-something.jpg"));

    let plans = plan_reorder_with_year(
        &[tmp.path()],
        &rust_default_opts(),
        &PlanOpts::default(),
        2025,
    )
    .expect("plan ok");

    assert_eq!(plans.len(), 1, "expected exactly 1 plan, got {:?}", plans);
    assert_eq!(
        plans[0].new_name.to_string_lossy(),
        "2025-12-01-IMG-something.jpg",
        "date should be moved to front"
    );
}

#[test]
fn reorder_skips_canonical() {
    let tmp = TempDir::new().expect("tempdir");
    touch(tmp.path().join("2025-12-01-IMG-something.jpg"));

    let plans = plan_reorder_with_year(
        &[tmp.path()],
        &rust_default_opts(),
        &PlanOpts::default(),
        2025,
    )
    .expect("plan ok");

    assert_eq!(
        plans.len(),
        0,
        "canonical file (date already at front) should be skipped, got {:?}",
        plans
    );
}

#[test]
fn reorder_skips_no_date() {
    let tmp = TempDir::new().expect("tempdir");
    touch(tmp.path().join("some-file-no-date.txt"));

    let plans = plan_reorder_with_year(
        &[tmp.path()],
        &rust_default_opts(),
        &PlanOpts::default(),
        2025,
    )
    .expect("plan ok");

    assert_eq!(
        plans.len(),
        0,
        "file with no date should be skipped, got {:?}",
        plans
    );
}

#[test]
fn reorder_uses_first_date() {
    let tmp = TempDir::new().expect("tempdir");
    touch(tmp.path().join("Meeting-2024-01-15-review-2023-11-20.mp4"));

    let plans = plan_reorder_with_year(
        &[tmp.path()],
        &rust_default_opts(),
        &PlanOpts::default(),
        2024,
    )
    .expect("plan ok");

    assert_eq!(plans.len(), 1, "expected exactly 1 plan, got {:?}", plans);
    assert!(
        plans[0]
            .new_name
            .to_string_lossy()
            .starts_with("2024-01-15"),
        "new name should start with first detected date, got: {}",
        plans[0].new_name.to_string_lossy()
    );
}

#[test]
fn reorder_moves_date_to_front_lower_case_mode() {
    let tmp = TempDir::new().expect("tempdir");
    touch(tmp.path().join("IMG-01-12-2025-Something.jpg"));

    let lower_opts = SlugOpts {
        separator: '-',
        case: CaseMode::Lower,
        split_camel: false,
    };

    let plans = plan_reorder_with_year(&[tmp.path()], &lower_opts, &PlanOpts::default(), 2025)
        .expect("plan ok");

    assert_eq!(plans.len(), 1, "expected exactly 1 plan, got {:?}", plans);
    assert!(
        plans[0]
            .new_name
            .to_string_lossy()
            .starts_with("2025-12-01"),
        "date prefix should be correct under lower case mode, got: {}",
        plans[0].new_name.to_string_lossy()
    );
}

#[test]
fn reorder_file_passed_as_root() {
    // Regression: passing a file path directly as root was silently skipped
    // because the root-skip guard fired on the file itself.
    let tmp = TempDir::new().expect("tempdir");
    let file = tmp.path().join("EngageMe-Survey-2026-05-22.pdf");
    touch(&file);

    let plans = plan_reorder_with_year(
        &[&file],
        &rust_default_opts(),
        &PlanOpts::default(),
        2026,
    )
    .expect("plan ok");

    assert_eq!(plans.len(), 1, "file passed as root should be planned, got {:?}", plans);
    assert!(
        plans[0].new_name.to_string_lossy().starts_with("2026-05-22"),
        "date should be moved to front, got: {}",
        plans[0].new_name.to_string_lossy()
    );
}

#[test]
fn rename_file_passed_as_root() {
    // Same root-skip regression for plan_with_year: file roots must be included.
    let tmp = TempDir::new().expect("tempdir");
    let file = tmp.path().join("IMG_2025_12_01_photo.jpg");
    touch(&file);

    let plans = plan_with_year(
        &[&file],
        &rust_default_opts(),
        &PlanOpts::default(),
        2025,
    )
    .expect("plan ok");

    assert_eq!(plans.len(), 1, "file passed as root should be planned, got {:?}", plans);
}
