#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Integration tests for `fren_date::merge_directories`. Ports the Python
//! `test_merge_directories` and `test_unique_file_name` cases.

use fren_date::{merge_directories, unique_file_name};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn touch(path: impl AsRef<Path>) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, b"").unwrap();
}

#[test]
fn test_unique_file_name_numeric_index() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("file.txt");

    // Doesn't exist → returned as-is.
    assert_eq!(unique_file_name(&path), path);

    fs::write(&path, b"").unwrap();
    let first = path.with_file_name("file_Copy.txt");
    assert_eq!(unique_file_name(&path), first);

    fs::write(&first, b"").unwrap();
    let second = path.with_file_name("file_Copy1.txt");
    assert_eq!(unique_file_name(&first), second);

    fs::write(&second, b"").unwrap();
    let third = path.with_file_name("file_Copy2.txt");
    assert_eq!(unique_file_name(&second), third);
}

#[test]
fn test_merge_directories_full() {
    // Mirrors Python's `test_merge_directories` from
    // `tests/test_files.py` exactly.
    let tmp = TempDir::new().unwrap();
    let target = tmp.path();

    // Pre-existing target tree.
    touch(target.join("2020").join("12").join("one.txt"));
    touch(target.join("2020").join("root.txt"));

    let other = target.join("other");
    touch(other.join("2020").join("root_Copy.txt"));
    touch(other.join("2020").join("12").join("one.txt"));
    touch(other.join("2020").join("12").join("two.txt"));
    touch(other.join("2021").join("01").join("three.txt"));

    let another = target.join("another");
    touch(another.join("2020").join("12").join("one.txt"));
    touch(another.join("2020").join("12").join("two.txt"));
    touch(another.join("2020").join("root_Copy.txt"));

    merge_directories(target, &[other.as_ref(), another.as_ref()], false).unwrap();

    let mut actual: Vec<String> = walk(target);
    actual.sort();
    actual.retain(|p| !p.starts_with("other") && !p.starts_with("another")); // empty dirs left behind

    let expected = vec![
        "2020/12/one.txt",
        "2020/12/one_Copy.txt",
        "2020/12/one_Copy1.txt",
        "2020/12/two.txt",
        "2020/12/two_Copy.txt",
        "2020/root.txt",
        "2020/root_Copy.txt",
        "2020/root_Copy1.txt",
        "2021/01/three.txt",
    ];
    assert_eq!(actual, expected);
}

fn walk(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    walk_inner(root, root, &mut out);
    out
}

fn walk_inner(root: &Path, current: &Path, out: &mut Vec<String>) {
    for entry in fs::read_dir(current).unwrap() {
        let entry = entry.unwrap();
        let p = entry.path();
        if p.is_dir() {
            walk_inner(root, &p, out);
        } else {
            out.push(p.strip_prefix(root).unwrap().to_string_lossy().into_owned());
        }
    }
}

#[test]
fn merge_dry_run_makes_no_changes() {
    let tmp = TempDir::new().unwrap();
    let target = tmp.path();
    touch(target.join("a.txt"));
    let source = target.join("src");
    touch(source.join("a.txt"));

    let report = merge_directories(target, &[source.as_ref()], true).unwrap();
    assert_eq!(report.moved.len(), 1);
    // Source file should still exist (dry run).
    assert!(source.join("a.txt").exists());
}

#[test]
fn merge_skips_ds_store() {
    let tmp = TempDir::new().unwrap();
    let target = tmp.path();
    let source = target.join("src");
    touch(source.join(".DS_Store"));
    touch(source.join("real.txt"));

    let report = merge_directories(target, &[source.as_ref()], false).unwrap();
    let moved_names: Vec<_> = report
        .moved
        .iter()
        .map(|m| m.from.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert!(!moved_names.iter().any(|n| n == ".DS_Store"));
    assert!(moved_names.iter().any(|n| n == "real.txt"));
}
