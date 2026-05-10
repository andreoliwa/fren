//! The internal sentinel character used by the slugify pipeline.
//!
//! A Unicode Private Use Area codepoint. PUA characters:
//!
//! - Are reserved by Unicode for private use - never assigned a public
//!   meaning, never appear in real filenames.
//! - Cannot collide with any user-chosen separator (CLI accepts only
//!   ASCII printable chars).
//! - Are stable under NFKC normalization.
//! - Survive transliteration in `slug-preserve`'s `fold_to_ascii_keep`
//!   because they're explicitly passed through as the "keep" character.

/// Sentinel character used as the internal separator throughout the
/// slugify-then-detect-dates pipeline. Substituted to the user's chosen
/// separator at the very end.
pub const SENTINEL: char = '\u{E000}';
