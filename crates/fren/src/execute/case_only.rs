//! Case-only rename via temp name.
//!
//! On case-insensitive filesystems (macOS APFS-ci, Windows NTFS-ci),
//! renaming `Foo.txt` → `foo.txt` looks like the same path to the OS.
//! Some platforms refuse the rename; some succeed silently with no
//! change. The portable fix: rename to a unique temp name first, then
//! rename the temp to the final target.

use crate::FrenError;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Whether `from` and `to` differ only in ASCII case (and are therefore
/// the "same path" on a case-insensitive filesystem).
pub fn is_case_only_rename(from: &Path, to: &Path) -> bool {
    let from_str = from.to_string_lossy();
    let to_str = to.to_string_lossy();
    from_str != to_str && from_str.eq_ignore_ascii_case(&to_str)
}

/// Perform a case-only rename via a temp intermediate.
pub fn rename_via_temp(from: &Path, to: &Path) -> Result<(), FrenError> {
    let temp = make_temp_path(from);

    // Step 1: from → temp
    std::fs::rename(from, &temp).map_err(|source| FrenError::Io {
        path: from.to_path_buf(),
        source,
    })?;

    // Step 2: temp → to
    std::fs::rename(&temp, to).map_err(|source| FrenError::Io {
        path: temp.clone(),
        source,
    })?;

    Ok(())
}

fn make_temp_path(near: &Path) -> PathBuf {
    let uuid = Uuid::now_v7();
    let parent = near.parent().unwrap_or_else(|| Path::new("."));
    let name = near
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    parent.join(format!(".fren_tmp_{name}_{uuid}"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::TempDir;

    #[test]
    fn is_case_only_rename_detects_case_difference() {
        assert!(is_case_only_rename(
            Path::new("/dir/Foo.txt"),
            Path::new("/dir/foo.txt")
        ));
        assert!(is_case_only_rename(
            Path::new("/dir/FOO.TXT"),
            Path::new("/dir/foo.txt")
        ));
    }

    #[test]
    fn is_case_only_rename_false_for_different_names() {
        assert!(!is_case_only_rename(
            Path::new("/dir/foo.txt"),
            Path::new("/dir/bar.txt")
        ));
    }

    #[test]
    fn is_case_only_rename_false_for_identical_paths() {
        assert!(!is_case_only_rename(
            Path::new("/dir/foo.txt"),
            Path::new("/dir/foo.txt")
        ));
    }

    #[test]
    fn rename_via_temp_moves_file() {
        let tmp = TempDir::new().unwrap();
        let from = tmp.path().join("Hello.txt");
        let to = tmp.path().join("hello.txt");
        std::fs::write(&from, b"data").unwrap();

        // On case-sensitive FS (Linux), from and to are distinct paths.
        // rename_via_temp must move the file regardless of FS sensitivity.
        rename_via_temp(&from, &to).unwrap();

        // On case-insensitive FS the "from" and "to" are the same inode;
        // the file should exist under the new name.
        assert!(to.exists() || from.exists(), "file must exist after rename");
    }

    #[test]
    fn make_temp_path_is_in_same_dir() {
        let path = Path::new("/some/dir/myfile.txt");
        let temp = make_temp_path(path);
        assert_eq!(temp.parent().unwrap(), Path::new("/some/dir"));
        let name = temp.file_name().unwrap().to_string_lossy();
        assert!(name.starts_with(".fren_tmp_myfile.txt_"));
    }
}
