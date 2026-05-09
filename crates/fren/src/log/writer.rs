//! JSONL transaction-log writer.

use crate::FrenError;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Trait for sinks that record rename events. The default implementation
/// is [`JsonlLogSink`]; tests can use [`NullLogSink`].
pub trait LogSink {
    /// Append one record. Errors are returned but the executor decides
    /// whether to continue.
    fn append(&mut self, record: &LogRecord) -> Result<(), FrenError>;
    /// Path of the underlying log file, if any. Used for the
    /// `ExecutionReport.log_path`.
    fn path(&self) -> Option<&Path>;
}

/// One record in the JSONL stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LogRecord {
    /// Batch header, written first.
    Batch {
        /// Schema version.
        #[serde(rename = "v")]
        v: u32,
        /// Batch UUID.
        id: Uuid,
        /// ISO timestamp.
        ts: String,
        /// Subcommand: `rename`, `merge`, etc.
        cmd: String,
        /// Original CLI args.
        args: Vec<String>,
        /// Working directory at invocation.
        cwd: PathBuf,
        /// `fren` version string.
        fren_version: String,
    },
    /// Per-rename record.
    Rename {
        /// Schema version.
        #[serde(rename = "v")]
        v: u32,
        /// Per-record timestamp.
        ts: String,
        /// Original path.
        from: PathBuf,
        /// New path.
        to: PathBuf,
        /// Item kind: `file`, `dir`, `symlink`.
        kind: String,
    },
    /// Completion marker.
    End {
        /// Schema version.
        #[serde(rename = "v")]
        v: u32,
        /// Per-record timestamp.
        ts: String,
        /// `ok` / `partial` / `error`.
        status: String,
        /// Renames applied.
        applied: usize,
        /// Plans skipped.
        skipped: usize,
        /// Errors encountered.
        errors: usize,
    },
}

/// JSONL log sink that appends to a file in
/// `${XDG_STATE_HOME:-~/.local/state}/fren/log/`.
pub struct JsonlLogSink {
    writer: BufWriter<File>,
    path: PathBuf,
}

impl JsonlLogSink {
    /// Open a new log file. The filename is
    /// `<ISO-timestamp>-<batch-uuid>.jsonl`. The directory is created if
    /// missing.
    pub fn open(log_dir: Option<&Path>, batch_id: Uuid, ts: &str) -> Result<Self, FrenError> {
        let dir = match log_dir {
            Some(p) => p.to_path_buf(),
            None => default_log_dir(),
        };
        std::fs::create_dir_all(&dir).map_err(|source| FrenError::Io {
            path: dir.clone(),
            source,
        })?;
        let filename = format!("{ts}-{batch_id}.jsonl");
        let path = dir.join(filename);
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|source| FrenError::Io {
                path: path.clone(),
                source,
            })?;
        Ok(Self {
            writer: BufWriter::new(file),
            path,
        })
    }
}

impl LogSink for JsonlLogSink {
    fn append(&mut self, record: &LogRecord) -> Result<(), FrenError> {
        let line = serde_json::to_string(record).map_err(|e| FrenError::InvalidInput(e.to_string()))?;
        writeln!(self.writer, "{line}").map_err(|source| FrenError::Io {
            path: self.path.clone(),
            source,
        })?;
        self.writer.flush().map_err(|source| FrenError::Io {
            path: self.path.clone(),
            source,
        })?;
        Ok(())
    }

    fn path(&self) -> Option<&Path> {
        Some(&self.path)
    }
}

/// No-op sink for tests / `--no-log`.
pub struct NullLogSink;

impl LogSink for NullLogSink {
    fn append(&mut self, _record: &LogRecord) -> Result<(), FrenError> {
        Ok(())
    }
    fn path(&self) -> Option<&Path> {
        None
    }
}

fn default_log_dir() -> PathBuf {
    if let Some(state) = std::env::var_os("XDG_STATE_HOME") {
        return PathBuf::from(state).join("fren").join("log");
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".local")
            .join("state")
            .join("fren")
            .join("log");
    }
    // Last-resort fallback: current dir.
    PathBuf::from(".fren-log")
}
