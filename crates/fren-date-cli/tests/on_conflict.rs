#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Integration tests for `--on-conflict` behavior.

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

/// Set up a fixture: `Report 2022-02-21.txt` (slugifies to `Report-2022-02-21.txt`)
/// plus a pre-existing `Report-2022-02-21.txt` that will collide with it.
fn collision_fixture() -> TempDir {
    let tmp = TempDir::new().unwrap();
    touch(tmp.path().join("Report 2022-02-21.txt"));
    touch(tmp.path().join("Report-2022-02-21.txt"));
    tmp
}

#[test]
fn on_conflict_number_default_does_not_abort() {
    let tmp = collision_fixture();
    let out = fren_bin()
        .args(["rename", "--apply", "--yes"])
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "expected success, got {}\nstderr: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    let copy1 = tmp.path().join("Report-2022-02-21-copy-1.txt");
    assert!(copy1.exists(), "-copy-1 variant should exist after number-default rename");
}

#[test]
fn on_conflict_abort_aborts() {
    let tmp = collision_fixture();
    let out = fren_bin()
        .args(["rename", "--on-conflict", "abort", "--apply", "--yes"])
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "expected failure with --on-conflict abort, got success"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("target already exists") || stderr.contains("already exists"),
        "expected 'already exists' in stderr, got: {stderr}"
    );
}

#[test]
fn on_conflict_number_explicit() {
    let tmp = collision_fixture();
    let out = fren_bin()
        .args(["rename", "--on-conflict", "number", "--apply", "--yes"])
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "expected success with --on-conflict number, got {}\nstderr: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    let copy1 = tmp.path().join("Report-2022-02-21-copy-1.txt");
    assert!(copy1.exists(), "-copy-1 variant should exist after explicit --on-conflict number");
}

#[test]
fn on_conflict_invalid_value() {
    let tmp = TempDir::new().unwrap();
    let out = fren_bin()
        .args(["rename", "--on-conflict", "skip"])
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "expected clap to reject --on-conflict skip"
    );
}
