//! Core data types describing a rename batch.

use std::ffi::OsString;
use std::path::PathBuf;
use uuid::Uuid;

/// What kind of filesystem item a plan refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    /// Regular file.
    File,
    /// Directory.
    Dir,
    /// Symbolic link (treated as a file; never followed).
    Symlink,
}

/// What kind of date was detected inside a filename.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateKind {
    /// Year-month only (e.g. `1975-08`).
    MonthOnly,
    /// Full calendar date (e.g. `2017-12-30`).
    DateOnly,
    /// Calendar date with time component (e.g. `2017-12-30T10-44-56`).
    DateTime,
}

/// A date span detected inside a filename. `byte_span` is recorded so a
/// future caller can locate the date and move it within the name.
#[derive(Debug, Clone)]
pub struct DetectedDate {
    /// Byte range (in the slugified pre-substitution string) where the
    /// date was found.
    pub byte_span: std::ops::Range<usize>,
    /// Parsed date/time.
    pub parsed: chrono::NaiveDateTime,
    /// Pendulum-style format string from `POSSIBLE_FORMATS`.
    pub original_format: &'static str,
    /// Granularity of the detected date.
    pub kind: DateKind,
}

/// A single planned rename operation. Produced by the planner; consumed by
/// the executor in bottom-up order.
#[derive(Debug, Clone)]
pub struct RenamePlan {
    /// Path of the item at planning time.
    pub original_path: PathBuf,
    /// Parent directory at planning time.
    pub parent: PathBuf,
    /// Original name (just the last path component).
    pub old_name: OsString,
    /// Target name (just the last component, in the new parent).
    pub new_name: OsString,
    /// Depth of the original path relative to the batch root. Used by the
    /// executor to enforce the bottom-up invariant (deeper paths first).
    pub depth: usize,
    /// What kind of item this is.
    pub kind: ItemKind,
    /// If a date was detected during slugification, captured here so the
    /// reorder phase can locate and move it.
    pub detected_date: Option<DetectedDate>,
    /// UUID grouping all plans of the same batch.
    pub batch_id: Uuid,
}
