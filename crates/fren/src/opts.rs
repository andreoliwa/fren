//! Option structs for the `fren` library entry points.

use crate::SlugOpts;
use std::path::PathBuf;

/// Conflict resolution policy when a rename target already exists or two
/// plans target the same path.
///
/// Currently only [`ConflictPolicy::Abort`] is functional; the other
/// variants are reserved for future expansion and are accepted by the
/// type but not yet implemented in the planner/executor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConflictPolicy {
    /// Stop the batch on conflict. No I/O is performed when a conflict is
    /// detected at planning time.
    #[default]
    Abort,
    /// Append the user separator + an integer to the target name until a
    /// free name is found.
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
    /// What to do on conflicts. Currently only `Abort` is functional.
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
