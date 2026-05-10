//! Execution report returned from `fren::execute`.

use crate::FrenError;
use std::path::PathBuf;
use uuid::Uuid;

/// Summary of an executed batch.
#[derive(Debug)]
pub struct ExecutionReport {
    /// Number of renames successfully applied.
    pub applied: usize,
    /// Number of plans skipped (conflict policy `Skip`, future-phase use).
    pub skipped: usize,
    /// Errors encountered. The current executor aborts on the first error,
    /// so this contains at most one entry.
    pub errors: Vec<FrenError>,
    /// Batch UUID for the run, also recorded in the transaction log.
    pub batch_id: Uuid,
    /// Path of the JSONL transaction log written for this batch (if any).
    pub log_path: Option<PathBuf>,
}
