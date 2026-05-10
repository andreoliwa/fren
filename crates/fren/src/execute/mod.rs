//! Execute a sorted plan vector. Reads the bottom-up invariant from
//! [`crate::plan::sort_bottom_up`]. Performs each rename via the
//! atomic primitives in [`atomic`] so that pre-existing targets are
//! never silently overwritten.

mod atomic;
mod case_only;

use crate::log::{LogRecord, LogSink, NullLogSink};
use crate::plan_types::ItemKind;
use crate::{ExecutionReport, FrenError, RenamePlan};
use chrono::Utc;
use uuid::Uuid;

/// Callback invoked after each successful rename during execution.
///
/// Implement this in the caller (e.g. the CLI) to stream rename output as
/// each operation completes, rather than buffering and printing after the
/// full batch. The library calls `on_rename` with the completed plan; the
/// implementation decides how (or whether) to display it.
///
/// A no-op implementation is provided via [`NullProgressSink`].
pub trait ProgressSink {
    /// Called immediately after a rename succeeds.
    fn on_rename(&mut self, plan: &RenamePlan);
}

/// No-op [`ProgressSink`]. Used by [`execute`] and [`execute_with_log`].
pub struct NullProgressSink;

impl ProgressSink for NullProgressSink {
    fn on_rename(&mut self, _plan: &RenamePlan) {}
}

/// Apply a sorted plan vector with no transaction logging.
///
/// Equivalent to [`execute_with_log`] using a [`NullLogSink`]. Convenience
/// for tests and quick scripts.
pub fn execute(plans: &[RenamePlan]) -> Result<ExecutionReport, FrenError> {
    let mut sink = NullLogSink;
    execute_with_log(plans, &mut sink)
}

/// Apply a sorted plan vector, recording every rename to `log_sink`.
pub fn execute_with_log(
    plans: &[RenamePlan],
    log_sink: &mut dyn LogSink,
) -> Result<ExecutionReport, FrenError> {
    execute_with_progress(plans, log_sink, &mut NullProgressSink)
}

/// Apply a sorted plan vector, recording to `log_sink` and calling
/// `progress` after each successful rename.
///
/// This is the core executor. [`execute`] and [`execute_with_log`] are
/// convenience wrappers that supply a [`NullProgressSink`].
pub fn execute_with_progress(
    plans: &[RenamePlan],
    log_sink: &mut dyn LogSink,
    progress: &mut dyn ProgressSink,
) -> Result<ExecutionReport, FrenError> {
    let batch_id = plans.first().map(|p| p.batch_id).unwrap_or_else(Uuid::nil);

    let mut applied = 0usize;
    let mut errors: Vec<FrenError> = Vec::new();

    for plan in plans {
        let from = &plan.original_path;
        let to_path = plan.parent.join(&plan.new_name);

        let outcome = match atomic::rename(from, &to_path) {
            Ok(()) => Ok(()),
            Err(FrenError::TargetExists(_)) if case_only::is_case_only_rename(from, &to_path) => {
                case_only::rename_via_temp(from, &to_path)
            }
            Err(e) => Err(e),
        };

        match outcome {
            Ok(()) => {
                applied += 1;
                progress.on_rename(plan);
                let kind = match plan.kind {
                    ItemKind::File => "file",
                    ItemKind::Dir => "dir",
                    ItemKind::Symlink => "symlink",
                };
                let record = LogRecord::Rename {
                    v: 1,
                    ts: Utc::now().to_rfc3339(),
                    from: from.clone(),
                    to: to_path,
                    kind: kind.to_string(),
                };
                // Log errors are noted but don't abort the batch.
                if let Err(e) = log_sink.append(&record) {
                    errors.push(e);
                }
            }
            Err(e) => {
                errors.push(e);
                break; // abort policy: stop on first failure
            }
        }
    }

    let log_path = log_sink.path().map(|p| p.to_path_buf());

    Ok(ExecutionReport {
        applied,
        skipped: 0,
        errors,
        batch_id,
        log_path,
    })
}
