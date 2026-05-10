//! Atomic rename primitive.
//!
//! Every rename must use a primitive that refuses to overwrite an
//! existing target. `std::fs::rename` is avoided directly because it
//! silently overwrites on Unix.
//!
//! Platform implementations:
//!
//! - Linux (kernel ≥ 3.15): `renameat2(RENAME_NOREPLACE)` - kernel-enforced
//!   atomicity; no TOCTOU window.
//! - All other platforms: explicit pre-check `to.exists()`, then
//!   `std::fs::rename`. Has a small TOCTOU window but acceptable for a
//!   single-user CLI with no concurrent writers.
//!
//! Stronger primitives for non-Linux platforms:
//! - macOS: `renamex_np(RENAME_EXCL)` (10.12+) or `link(2)`+`unlink(2)`.
//! - Windows: `MoveFileExW` without `MOVEFILE_REPLACE_EXISTING`.

use crate::FrenError;
use std::path::Path;

/// Rename `from` to `to`, refusing to overwrite if `to` already exists.
pub fn rename(from: &Path, to: &Path) -> Result<(), FrenError> {
    #[cfg(target_os = "linux")]
    {
        rename_linux(from, to)
    }
    #[cfg(not(target_os = "linux"))]
    {
        rename_fallback(from, to)
    }
}

/// Linux: use `renameat2(RENAME_NOREPLACE)` for kernel-enforced no-overwrite.
#[cfg(target_os = "linux")]
fn rename_linux(from: &Path, to: &Path) -> Result<(), FrenError> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    // RENAME_NOREPLACE: fail with EEXIST if target already exists.
    // Available since Linux 3.15; glibc exposes it via renameat2(2).
    const RENAME_NOREPLACE: libc::c_uint = 1;
    const AT_FDCWD: libc::c_int = -100;

    let from_c = CString::new(from.as_os_str().as_bytes()).map_err(|_| {
        FrenError::InvalidInput(format!("path contains null byte: {}", from.display()))
    })?;
    let to_c = CString::new(to.as_os_str().as_bytes()).map_err(|_| {
        FrenError::InvalidInput(format!("path contains null byte: {}", to.display()))
    })?;

    // SAFETY: renameat2 is a raw syscall; all pointers are valid CStrings
    // for the duration of the call.
    let ret = unsafe {
        libc::syscall(
            libc::SYS_renameat2,
            AT_FDCWD,
            from_c.as_ptr(),
            AT_FDCWD,
            to_c.as_ptr(),
            RENAME_NOREPLACE,
        )
    };

    if ret == 0 {
        return Ok(());
    }

    let err = std::io::Error::last_os_error();

    // ENOSYS: kernel too old (< 3.15) or filesystem doesn't support it.
    // Fall back to the pre-check approach so old kernels still work.
    if err.raw_os_error() == Some(libc::ENOSYS) {
        return rename_fallback(from, to);
    }

    // EEXIST: target already exists - that's our TargetExists sentinel.
    if err.raw_os_error() == Some(libc::EEXIST) {
        return Err(FrenError::TargetExists(to.to_path_buf()));
    }

    Err(FrenError::Io {
        path: from.to_path_buf(),
        source: err,
    })
}

/// Fallback for non-Linux or kernels that don't support renameat2.
///
/// Pre-check + `std::fs::rename`. Has a small TOCTOU window.
fn rename_fallback(from: &Path, to: &Path) -> Result<(), FrenError> {
    if to.exists() {
        return Err(FrenError::TargetExists(to.to_path_buf()));
    }
    std::fs::rename(from, to).map_err(|source| FrenError::Io {
        path: from.to_path_buf(),
        source,
    })
}
