//! Transaction log: append-only JSONL files recording every rename batch.
//!
//! Each batch produces one file at
//! `${XDG_STATE_HOME:-~/.local/state}/fren/log/<ISO-timestamp>-<batch-uuid>.jsonl`.
//!
//! Format:
//!
//! - Line 1: batch header `{"v":1,"type":"batch",...}`
//! - Lines 2..N: per-rename `{"v":1,"type":"rename",...}`
//! - Final line: completion marker `{"v":1,"type":"end",...}`
//!
//! Future `fren undo` and `fren history` commands consume these files.

mod writer;

pub use writer::{JsonlLogSink, LogRecord, LogSink, NullLogSink};
