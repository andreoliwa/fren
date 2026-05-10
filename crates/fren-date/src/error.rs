//! Error type for the `fren` library.
//!
//! All fallible public functions return `Result<T, FrenError>`. The library
//! never panics on user input.

use std::path::PathBuf;
use thiserror::Error;

/// Top-level error for the `fren` library.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum FrenError {
    /// I/O error from the filesystem layer.
    #[error("I/O error at {path}: {source}")]
    Io {
        /// Path the error relates to (when known).
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// A rename target already exists (conflict policy = abort).
    #[error("target already exists: {0}")]
    TargetExists(PathBuf),

    /// Two plans within the same batch want to rename to the same target.
    #[error("within-batch collision: {a} and {b} both want to rename to {target}")]
    WithinBatchCollision {
        /// First plan's source path.
        a: PathBuf,
        /// Second plan's source path.
        b: PathBuf,
        /// The target both plans contend for.
        target: PathBuf,
    },

    /// A user-facing input was invalid (bad path, bad option value).
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// A feature is reserved for a future release.
    #[error("not yet implemented: {0}")]
    NotYetImplemented(String),
}
