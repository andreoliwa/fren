//! Option structs for the `fren` library entry points.

use crate::SlugOpts;
use std::path::PathBuf;

/// Conflict resolution policy when a rename target already exists or two
/// plans target the same path.
///
/// [`ConflictPolicy::Abort`] and [`ConflictPolicy::Number`] are fully
/// functional. [`ConflictPolicy::Skip`] and [`ConflictPolicy::Merge`] are
/// reserved for future expansion and currently return
/// [`crate::FrenError::NotYetImplemented`] when triggered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConflictPolicy {
    /// Stop the batch on conflict. No I/O is performed when a conflict is
    /// detected at planning time.
    Abort,
    /// Append `-copy-{n}` to the stem of the conflicting target name (before
    /// the extension) until a free name is found, starting at n=1. The chosen
    /// name is guaranteed not to overwrite any file already on disk or any
    /// other plan in the same batch.
    ///
    /// Example: `report.tar.gz` becomes `report.tar-copy-1.gz`,
    /// `report.tar-copy-2.gz`, etc.
    ///
    /// This is the default policy, matching the `fren rename` CLI default.
    #[default]
    Number,
    /// Skip just the conflicting plan; continue the batch.
    Skip,
    /// For directory-vs-directory conflicts, recursively merge contents
    /// (file conflicts inside fall back to `Number`). For file-vs-file
    /// conflicts, behaves like `Abort`.
    Merge,
}

/// Options for planning a rename batch.
#[derive(Debug, Clone, Default)]
pub struct PlanOpts {
    /// Paths to exclude.
    pub exclude: Vec<PathBuf>,
    /// Whether to traverse subdirectories.
    pub recursive: bool,
    /// What to do on conflicts. Defaults to `Number` (rename conflicting targets
    /// to `-copy-{n}` variants). `Abort` and `Number` are fully functional;
    /// `Skip` and `Merge` return `NotYetImplemented`.
    pub on_conflict: ConflictPolicy,
}

/// Top-level options for `fren::rename` (the high-level convenience entry).
#[derive(Debug, Clone, Default)]
pub struct RenameOpts {
    /// Slugify options (separator, case).
    pub slugify: SlugOpts,
    /// Planning options (recursion, exclusion, conflict policy).
    pub plan: PlanOpts,
    /// `true` to actually execute the renames; `false` for dry-run (default).
    pub apply: bool,
}
