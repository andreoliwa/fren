//! Atomic rename primitive.
//!
//! Every rename must use a primitive that refuses to overwrite an
//! existing target. `std::fs::rename` is avoided directly because it
//! silently overwrites on Unix.
//!
//! Current implementation: explicit pre-check `to.exists()`, then
//! `std::fs::rename`. This has a small TOCTOU window but is acceptable
//! on the hot path (single-user CLI, no concurrent writers expected).
//!
//! Stronger primitives that could replace this in the future:
//!
//! - Linux: `renameat2(RENAME_NOREPLACE)` for kernel-enforced atomicity.
//! - macOS: `renamex_np(RENAME_EXCL)` (10.12+) or the `link(2)`+`unlink(2)` trick.
//! - Windows: `MoveFileExW` without `MOVEFILE_REPLACE_EXISTING`.

use crate::FrenError;
use std::path::Path;

/// Rename `from` to `to`, refusing to overwrite if `to` already exists.
pub fn rename(from: &Path, to: &Path) -> Result<(), FrenError> {
    if to.exists() {
        return Err(FrenError::TargetExists(to.to_path_buf()));
    }
    std::fs::rename(from, to).map_err(|source| FrenError::Io {
        path: from.to_path_buf(),
        source,
    })
}
