//! `fren` - file renamer that understands dates.
//!
//! Library crate. The CLI binary lives in `fren-cli` and is a thin consumer
//! of this library.
//!
//! ## Quick example
//!
//! ```
//! use fren_date::{slugify_camel_iso_with_year, SlugOpts};
//! use slug_preserve::CaseMode;
//!
//! let legacy_python = SlugOpts {
//!     separator: '_',
//!     case: CaseMode::Capitalize,
//!     split_camel: true,
//! };
//! let out = slugify_camel_iso_with_year("Hello World 2024-01-15.txt", &legacy_python, 2024);
//! assert!(out.starts_with("Hello"));
//! ```
//!
//! ## Library discipline
//!
//! This crate must obey:
//! - No `panic!` on user input - return `Err(FrenError::*)`
//! - No `println!`/`eprintln!` - output flows through trait objects
//! - No global state - all configuration via opts structs

#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]

pub use slug_preserve::{CaseMode, SlugOpts};

mod error;
pub use error::FrenError;

mod opts;
pub use opts::{ConflictPolicy, PlanOpts, RenameOpts};

mod plan_types;
pub use plan_types::{DateKind, DetectedDate, ItemKind, RenamePlan};

mod report;
pub use report::ExecutionReport;

mod date;
mod execute;
mod log;
mod merge;
mod plan;
mod slugify;

pub use execute::{
    execute, execute_with_log, execute_with_progress, NullProgressSink, ProgressSink,
};
pub use log::{JsonlLogSink, LogRecord, LogSink, NullLogSink};
pub use merge::{merge_directories, unique_file_name, MergeMove, MergeReport};
pub use plan::{plan, plan_reorder, plan_reorder_with_year, plan_with_year, sort_bottom_up};
pub use slugify::{slugify_camel_iso, slugify_camel_iso_detect, slugify_camel_iso_with_year};

use std::path::Path;

/// High-level entry point: plan + (optionally) execute a rename batch.
///
/// If `opts.apply` is `false`, returns the plan vector wrapped in an
/// `ExecutionReport` with `applied = 0`; the caller is responsible for
/// displaying the dry-run preview.
///
/// If `opts.apply` is `true`, executes the plan and returns a real report.
pub fn rename(
    roots: &[&Path],
    opts: &RenameOpts,
) -> Result<(Vec<RenamePlan>, ExecutionReport), FrenError> {
    let plans = plan(roots, &opts.slugify, &opts.plan)?;
    let report = if opts.apply {
        execute(&plans)?
    } else {
        ExecutionReport {
            applied: 0,
            skipped: 0,
            errors: Vec::new(),
            batch_id: plans
                .first()
                .map(|p| p.batch_id)
                .unwrap_or_else(uuid::Uuid::nil),
            log_path: None,
        }
    };
    Ok((plans, report))
}

/// Slugify a single string with the given options.
///
/// Wrapper around [`slug_preserve::slugify`]. See that crate's docs for
/// details. This is re-exported so library users don't need to depend on
/// `slug-preserve` directly.
#[must_use]
pub fn slugify_str(input: &str, opts: &SlugOpts) -> String {
    slug_preserve::slugify(input, opts)
}
