//! Date format table.
//!
//! Mirrors Python's `POSSIBLE_FORMATS` constant from `src/fren.py`. Each
//! format is described by its Pendulum-style template (e.g. `DD_MM_YYYY`)
//! and a granularity tag (`MonthOnly`, `DateOnly`, `DateTime`).
//!
//! The table is keyed off `_` for human readability; the parser converts
//! the underscore separators into the runtime sentinel before matching.

use crate::DateKind;

/// One date format the parser will try.
#[derive(Debug, Clone, Copy)]
pub struct FormatSpec {
    /// Pendulum-style template using `_` placeholders for separators.
    pub template: &'static str,
    /// Granularity of dates parsed by this template.
    pub kind: DateKind,
    /// Whether the template carries a 4-digit year (`true`) or 2-digit (`false`).
    pub has_century: bool,
}

/// Determine output `DateKind` from an input format template.
#[must_use]
pub const fn format_kind(template: &'static str) -> DateKind {
    let bytes = template.as_bytes();
    let mut i = 0;
    let mut has_h = false;
    let mut has_d = false;
    while i < bytes.len() {
        if bytes[i] == b'H' {
            has_h = true;
        }
        if bytes[i] == b'D' {
            has_d = true;
        }
        i += 1;
    }
    if has_h {
        DateKind::DateTime
    } else if has_d {
        DateKind::DateOnly
    } else {
        DateKind::MonthOnly
    }
}

const fn has_century(template: &'static str) -> bool {
    let bytes = template.as_bytes();
    let mut i = 0;
    let mut yyy_count = 0u8;
    while i < bytes.len() {
        if bytes[i] == b'Y' {
            yyy_count += 1;
            if yyy_count >= 4 {
                return true;
            }
        } else {
            yyy_count = 0;
        }
        i += 1;
    }
    false
}

const fn spec(template: &'static str) -> FormatSpec {
    FormatSpec {
        template,
        kind: format_kind(template),
        has_century: has_century(template),
    }
}

/// Ordered list of format templates the parser tries, mirroring Python.
///
/// **Order matters.** Human-first formats (`DD_MM_YYYY`) are tried before
/// inverted/ISO formats (`YYYY_MM_DD`) so that ambiguous strings like
/// `01_12_2019` are interpreted day-first.
pub const POSSIBLE_FORMATS: &[FormatSpec] = &[
    // Human formats first
    spec("MM_YYYY"),
    spec("DD_MM_YYYY"),
    spec("DD_MM_YY"),
    spec("DDMMYYYY"),
    spec("DDMMYY"),
    spec("DD_MM_YYYY_HH_mm_ss"),
    spec("DD_MM_YY_HH_mm_ss"),
    spec("DD_MM_YYYY_HH_mm"),
    spec("DDMMYYYYHHmm"),
    // Then inverted formats
    spec("YYYY_MM"),
    spec("YYYY_MM_DD"),
    spec("YYYYMMDD"),
    spec("YY_MM_DD_HH_mm_ss"),
    spec("YYYY_MM_DD_HH_mm_ss"),
    spec("YYYY_MM_DD_HH_mm"),
    spec("YYYY_MM_DD_HHmm"),
    spec("YYYYMMDDHHmmss"),
    spec("YYYYMMDD_HHmmss"),
    spec("YYYYMMDDHHmm"),
];
