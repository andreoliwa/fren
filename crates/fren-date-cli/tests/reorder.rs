#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Integration tests for `fren reorder`.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn fren_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_fren"))
}

fn touch(path: impl AsRef<std::path::Path>) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, b"x").unwrap();
}

#[test]
fn reorder_apply_moves_date_to_front() {
    let tmp = TempDir::new().unwrap();
    touch(tmp.path().join("IMG-01-12-2025-Something-bla.jpg"));

    let out = fren_bin()
        .args(["reorder", "--apply", "--yes"])
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "expected success, got {}\nstderr: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        tmp.path().join("2025-12-01-IMG-Something-bla.jpg").exists(),
        "expected 2025-12-01-IMG-Something-bla.jpg to exist"
    );
}

#[test]
fn reorder_dry_run_prints_plan() {
    let tmp = TempDir::new().unwrap();
    touch(tmp.path().join("IMG-01-12-2025-something.jpg"));

    let out = fren_bin()
        .args(["reorder"])
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "expected success, got {}\nstderr: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    // Original file still exists (dry-run does not modify)
    assert!(
        tmp.path().join("IMG-01-12-2025-something.jpg").exists(),
        "expected original file unchanged after dry-run"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let combined = format!("{stdout}{stderr}");
    assert!(
        combined.contains("Would rename") || combined.contains("Would reorder"),
        "expected 'Would rename' or 'Would reorder' in output, got: {combined}"
    );
}

#[test]
fn reorder_skips_already_canonical() {
    let tmp = TempDir::new().unwrap();
    touch(tmp.path().join("2025-12-01-IMG-something.jpg"));

    let out = fren_bin()
        .args(["reorder"])
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "expected success, got {}\nstderr: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("All names already in canonical form."),
        "expected 'All names already in canonical form.' in stdout, got: {stdout}"
    );
}

#[test]
fn reorder_on_conflict_numbering() {
    let tmp = TempDir::new().unwrap();
    // Source: will reorder to "2025-12-01-IMG.jpg"
    touch(tmp.path().join("IMG-01-12-2025.jpg"));
    // Pre-existing target: collision
    touch(tmp.path().join("2025-12-01-IMG.jpg"));

    let out = fren_bin()
        .args(["reorder", "--apply", "--yes"])
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "expected success with default --on-conflict number, got {}\nstderr: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    // A numbered variant starting with "2025-12-01-IMG" but not the exact collision target
    let numbered = fs::read_dir(tmp.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .find(|name| name.starts_with("2025-12-01-IMG") && name != "2025-12-01-IMG.jpg");
    assert!(
        numbered.is_some(),
        "expected a numbered variant of '2025-12-01-IMG.jpg' to exist"
    );
}

/// reorder slugifies a date-less file whose name is not yet canonical.
/// A file with spaces and no date must be slugified the same way `rename` would.
#[test]
fn reorder_slugifies_dateless_file() {
    let tmp = TempDir::new().unwrap();
    touch(tmp.path().join("My Photo from the trip.jpg"));

    let out = fren_bin()
        .args(["reorder", "--apply", "--yes"])
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "expected success, got {}\nstderr: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    // The original spaced filename must be gone and no files may still contain
    // a space - the slug pipeline replaces spaces with the separator.
    let names: Vec<String> = fs::read_dir(tmp.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        !tmp.path().join("My Photo from the trip.jpg").exists(),
        "original file should have been renamed"
    );
    assert!(
        names.iter().all(|n| !n.contains(' ')),
        "no file name in the dir should contain a space after reorder; found: {names:?}"
    );
    assert!(
        tmp.path().join("My-Photo-from-the-trip.jpg").exists(),
        "expected My-Photo-from-the-trip.jpg"
    );
}

/// reorder skips a date-less file that is already in canonical slug form.
#[test]
fn reorder_skips_dateless_canonical() {
    let tmp = TempDir::new().unwrap();
    // This name is already canonical - `rename` reports "All names already in
    // canonical form." for it and reorder must do the same.
    touch(tmp.path().join("my-photo-from-the-trip.jpg"));

    let out = fren_bin()
        .args(["reorder"])
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "expected success, got {}\nstderr: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("All names already in canonical form."),
        "expected 'All names already in canonical form.' in stdout, got: {stdout}"
    );
}

/// reorder and rename must produce the same target name for any date-less file.
#[test]
fn reorder_matches_rename_for_dateless() {
    let original = "My Photo from the trip.jpg";

    // Run `rename` in one temp dir.
    let rename_dir = TempDir::new().unwrap();
    touch(rename_dir.path().join(original));
    let out = fren_bin()
        .args(["rename", "--apply", "--yes"])
        .arg(rename_dir.path())
        .output()
        .unwrap();
    assert!(out.status.success(), "rename failed: {:?}", out.status);

    // Run `reorder` in a separate temp dir.
    let reorder_dir = TempDir::new().unwrap();
    touch(reorder_dir.path().join(original));
    let out = fren_bin()
        .args(["reorder", "--apply", "--yes"])
        .arg(reorder_dir.path())
        .output()
        .unwrap();
    assert!(out.status.success(), "reorder failed: {:?}", out.status);

    // Collect resulting filenames (excluding the original if it somehow remained).
    let rename_result: Vec<String> = fs::read_dir(rename_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    let reorder_result: Vec<String> = fs::read_dir(reorder_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();

    assert_eq!(
        rename_result, reorder_result,
        "rename and reorder must produce the same name for a date-less file"
    );
}

/// Verify that --on-conflict is wired correctly: with abort policy and a
/// pre-existing conflict target, the command fails even in dry-run (the
/// planner raises an error before any rename occurs). This confirms that
/// conflict_policy is passed consistently from the CLI into plan_reorder.
#[test]
fn reorder_dry_run_respects_on_conflict() {
    let tmp = TempDir::new().unwrap();
    // Source: will reorder to "2025-12-01-IMG.jpg"
    touch(tmp.path().join("IMG-01-12-2025.jpg"));
    // Pre-existing target: collision
    touch(tmp.path().join("2025-12-01-IMG.jpg"));

    // Dry-run with --on-conflict abort should fail: the planner detects the
    // pre-existing target and returns an error under Abort policy, which is
    // the expected behavior - conflict_policy is threaded into plan_reorder.
    let out = fren_bin()
        .args(["reorder", "--on-conflict", "abort"])
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "expected failure with --on-conflict abort and a collision, got success"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("target already exists") || stderr.contains("already exists"),
        "expected 'already exists' in stderr, got: {stderr}"
    );
    // Source and pre-existing files unchanged (no rename happened)
    assert!(
        tmp.path().join("IMG-01-12-2025.jpg").exists(),
        "source file should be unchanged"
    );
    assert!(
        tmp.path().join("2025-12-01-IMG.jpg").exists(),
        "pre-existing file should be unchanged"
    );
}
