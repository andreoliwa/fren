//! Date detection and ISO conversion.
//!
//! Ports the Python `try_date()` + `POSSIBLE_FORMATS` logic to Rust. The
//! detection runs over a slugified-with-sentinel string (see
//! `crates/fren/src/slugify/sentinel.rs`); detected dates are emitted in
//! ISO 8601 form wrapped in sentinel markers so the surrounding pipeline
//! can collapse them.

mod formats;
mod parser;

pub use parser::{detect_and_replace, detect_and_replace_with_span};
