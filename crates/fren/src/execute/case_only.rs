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
