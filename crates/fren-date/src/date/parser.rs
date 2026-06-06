//! Date parsing and ISO conversion.
//!
//! Ports the Python `try_date()` closure from `src/fren.py`. Strategy:
//!
//! 1. The slugified input has its date-bearing substrings substituted to use
//!    the runtime sentinel as the separator. We rewrite the
//!    underscore-keyed format templates to use the sentinel before matching.
//! 2. For each candidate substring (matched by the date regex), try every
//!    format whose **string length equals** the substring's length. Length
//!    parity is the strictness guard that keeps Pendulum's lenient parser
//!    from returning wrong dates; chrono is also lenient on numeric widths.
//! 3. On a successful parse, emit ISO output:
//!    - `MonthOnly` → `YYYY-MM`
//!    - `DateOnly` → `YYYY-MM-DD`
//!    - `DateTime` → `YYYY-MM-DDTHH-mm-ss`  (note hyphen before HH per Python)
//! 4. For 2-digit-year formats, treat years > (current year + 10) as
//!    belonging to the previous century (the Python "1929-2029" rule).

use crate::date::formats::{FormatSpec, POSSIBLE_FORMATS};
use crate::DateKind;
use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime};
use regex::Regex;
use std::sync::OnceLock;

/// Regex matching a candidate date span: starts and ends with a digit, may
/// contain digits, `-`, `_`, `.` in between. Mirrors Python's
/// `REGEX_DATE_TIME = r"([0-9][0-9-_\.]+[0-9])"`.
fn date_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // The slug pipeline normalizes most separators into the sentinel,
        // so in practice the candidate spans use `_`-or-sentinel as the
        // internal separator. We accept all of `-_.` for forward
        // compatibility with raw inputs that might contain dots.
        #[allow(clippy::expect_used)]
        // OK: regex literal known good at compile time, never user input.
        Regex::new(r"([0-9][0-9\-_\.]+[0-9])").expect("static date regex compiles")
    })
}

/// Zero-pad unpadded date candidates and return them in priority order for
/// `try_date` to attempt.
///
/// - ISO-ordered (`YYYY_M_D` etc.): unambiguous → one candidate `YYYY_MM_DD`.
/// - Human-ordered (`D_M_YYYY` etc.): ambiguous → two candidates in order:
///   1. `DD_MM_YYYY` (day/month/year - preferred)
///   2. `MM_DD_YYYY` (month/day/year - fallback if first yields invalid date)
///
/// Returns an empty slice when the candidate is already fully padded or does
/// not match any unpadded shape (the normal path handles it).
fn pad_unpadded_date(candidate: &str) -> impl Iterator<Item = String> + use<'_> {
    fn re_iso() -> &'static Regex {
        static RE: OnceLock<Regex> = OnceLock::new();
        RE.get_or_init(|| {
            #[allow(clippy::expect_used)]
            Regex::new(r"^(\d{4})_(\d{1,2})_(\d{1,2})$").expect("static pad-iso regex compiles")
        })
    }
    fn re_human() -> &'static Regex {
        static RE: OnceLock<Regex> = OnceLock::new();
        RE.get_or_init(|| {
            #[allow(clippy::expect_used)]
            Regex::new(r"^(\d{1,2})_(\d{1,2})_(\d{4})$")
                .expect("static pad-human regex compiles")
        })
    }

    // Use a small fixed-size array so no heap allocation is needed.
    let mut out: [Option<String>; 2] = [None, None];

    if let Some(caps) = re_iso().captures(candidate) {
        let y = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let m = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        let d = caps.get(3).map(|m| m.as_str()).unwrap_or("");
        if m.len() != 2 || d.len() != 2 {
            out[0] = Some(format!("{y}_{m:0>2}_{d:0>2}"));
        }
    } else if let Some(caps) = re_human().captures(candidate) {
        let a = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let b = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        let y = caps.get(3).map(|m| m.as_str()).unwrap_or("");
        if a.len() != 2 || b.len() != 2 {
            // D/M/Y first, M/D/Y as fallback
            out[0] = Some(format!("{a:0>2}_{b:0>2}_{y}"));
            out[1] = Some(format!("{b:0>2}_{a:0>2}_{y}"));
        }
    }

    out.into_iter().flatten()
}

/// Try `effective` against every format whose template length matches.
fn try_date_effective(effective: &str, current_year: i32) -> Option<(String, DateKind)> {
    for spec in POSSIBLE_FORMATS {
        if spec.template.len() != effective.len() {
            continue;
        }
        if let Some((dt, kind)) = parse_with_template(effective, spec, current_year) {
            return Some((format_iso(dt, kind), kind));
        }
    }
    None
}

/// Try to parse `candidate` as one of the known formats. Returns
/// `Some((iso_string, kind))` on success, `None` if no format matches.
///
/// `candidate` should already be using `_` as its internal separator
/// (which it will be if the slugify pipeline used `_` as the sentinel,
/// or if the caller pre-normalized).
///
/// Unpadded dates (`YYYY_M_D`, `D_M_YYYY`, etc.) are zero-padded before the
/// format-table lookup. For human-ordered ambiguous forms (`D_M_YYYY`),
/// D/M/Y is tried first; M/D/Y is the fallback when D/M yields an invalid date.
#[must_use]
pub fn try_date(candidate: &str, current_year: i32) -> Option<(String, DateKind)> {
    let mut pads = pad_unpadded_date(candidate).peekable();
    if pads.peek().is_some() {
        for padded in pads {
            if let Some(result) = try_date_effective(&padded, current_year) {
                return Some(result);
            }
        }
        return None;
    }
    try_date_effective(candidate, current_year)
}

fn parse_with_template(
    candidate: &str,
    spec: &FormatSpec,
    current_year: i32,
) -> Option<(NaiveDateTime, DateKind)> {
    let parts = split_by_template(candidate, spec.template)?;
    let year = parts.year?;
    let mut year = year as i32;

    if !spec.has_century {
        // 2-digit year. We have a raw 0..=99. Default to interpreting it
        // as 20YY, then apply the Python "subtract 100 if year > current+10"
        // rule to push it back to 19YY when appropriate.
        year += 2000;
        let next_ten = current_year + 10;
        if year > next_ten {
            year -= 100;
        }
    }

    let month = parts.month.unwrap_or(1);
    let day = parts.day.unwrap_or(1);
    let hour = parts.hour.unwrap_or(0);
    let minute = parts.minute.unwrap_or(0);
    let second = parts.second.unwrap_or(0);

    let date = NaiveDate::from_ymd_opt(year, month, day)?;
    let time = NaiveTime::from_hms_opt(hour, minute, second)?;
    Some((NaiveDateTime::new(date, time), spec.kind))
}

#[derive(Default)]
struct ParsedParts {
    year: Option<u32>,
    month: Option<u32>,
    day: Option<u32>,
    hour: Option<u32>,
    minute: Option<u32>,
    second: Option<u32>,
}

/// Walk `candidate` and `template` in lockstep, extracting each Pendulum
/// token's numeric value. Templates use:
///
/// - `YYYY` → 4-digit year
/// - `YY`   → 2-digit year (interpreted as 19YY for backward years; caller
///   may shift via the century rule)
/// - `MM`   → 2-digit month
/// - `DD`   → 2-digit day
/// - `HH`   → 2-digit 24-hour
/// - `mm`   → 2-digit minute
/// - `ss`   → 2-digit second
/// - any other char → must match candidate exactly (separator)
fn split_by_template(candidate: &str, template: &str) -> Option<ParsedParts> {
    let cb = candidate.as_bytes();
    let tb = template.as_bytes();
    let mut parts = ParsedParts::default();
    let mut ci = 0usize;
    let mut ti = 0usize;

    while ti < tb.len() {
        let t = tb[ti];
        if t == b'Y' {
            let n = run_len(tb, ti, b'Y');
            let raw = read_digits(cb, ci, n)?;
            ci += n;
            ti += n;
            // 4-digit year stored as-is; 2-digit year stored as raw (caller
            // applies century rule). chrono accepts only i32 → we cast at
            // use site.
            parts.year = Some(raw);
        } else if t == b'M' {
            let n = run_len(tb, ti, b'M');
            let raw = read_digits(cb, ci, n)?;
            ci += n;
            ti += n;
            parts.month = Some(raw);
        } else if t == b'D' {
            let n = run_len(tb, ti, b'D');
            let raw = read_digits(cb, ci, n)?;
            ci += n;
            ti += n;
            parts.day = Some(raw);
        } else if t == b'H' {
            let n = run_len(tb, ti, b'H');
            let raw = read_digits(cb, ci, n)?;
            ci += n;
            ti += n;
            parts.hour = Some(raw);
        } else if t == b'm' {
            let n = run_len(tb, ti, b'm');
            let raw = read_digits(cb, ci, n)?;
            ci += n;
            ti += n;
            parts.minute = Some(raw);
        } else if t == b's' {
            let n = run_len(tb, ti, b's');
            let raw = read_digits(cb, ci, n)?;
            ci += n;
            ti += n;
            parts.second = Some(raw);
        } else {
            // literal separator must match
            if ci >= cb.len() || cb[ci] != t {
                return None;
            }
            ci += 1;
            ti += 1;
        }
    }
    if ci != cb.len() {
        return None;
    }
    Some(parts)
}

fn run_len(bytes: &[u8], start: usize, ch: u8) -> usize {
    let mut n = 0;
    while start + n < bytes.len() && bytes[start + n] == ch {
        n += 1;
    }
    n
}

fn read_digits(bytes: &[u8], start: usize, n: usize) -> Option<u32> {
    if start + n > bytes.len() {
        return None;
    }
    let slice = &bytes[start..start + n];
    if !slice.iter().all(u8::is_ascii_digit) {
        return None;
    }
    // ASCII-only slice → safe UTF-8.
    #[allow(clippy::expect_used)]
    let s = std::str::from_utf8(slice).expect("ASCII digits are valid UTF-8");
    s.parse().ok()
}

fn format_iso(dt: NaiveDateTime, kind: DateKind) -> String {
    match kind {
        DateKind::MonthOnly => format!("{:04}-{:02}", dt.year(), dt.month()),
        DateKind::DateOnly => dt.format("%Y-%m-%d").to_string(),
        // Python uses `T` separator with hyphens between hms components
        // (rather than colons) so the result is a valid filename on all
        // platforms. We match exactly.
        DateKind::DateTime => dt.format("%Y-%m-%dT%H-%M-%S").to_string(),
    }
}

/// Split `candidate` on `_`, then slide windows of every valid size over the
/// segments and call `try_date` on each rejoined window. Returns
/// `Some((prefix, iso, suffix))` for the first (leftmost, then shortest)
/// window that parses as a valid date, where `prefix` and `suffix` are the
/// rejoined segments outside the window (e.g. `"3_"` and `"_810"`).
/// Returns `None` if no window yields a valid date.
///
/// This handles both extra leading tokens (`3_29_5_2026` → window `29_5_2026`)
/// and extra trailing tokens (`20260406_225315_810` → window `20260406_225315`,
/// suffix `_810`), as well as arbitrary surrounding digits.
fn try_date_in_windows(candidate: &str, current_year: i32) -> Option<(String, String, String)> {
    let segments: Vec<&str> = candidate.split('_').collect();
    let n = segments.len();
    // Try windows from largest to smallest so the most-specific match wins
    // when multiple window sizes would succeed (e.g. datetime before date).
    // Within each size, scan left-to-right (leftmost win).
    for size in (1..=n).rev() {
        for start in 0..=(n - size) {
            let window = segments[start..start + size].join("_");
            if let Some((iso, _)) = try_date(&window, current_year) {
                let prefix = if start == 0 {
                    String::new()
                } else {
                    format!("{}_", segments[..start].join("_"))
                };
                let suffix = if start + size == n {
                    String::new()
                } else {
                    format!("_{}", segments[start + size..].join("_"))
                };
                return Some((prefix, iso, suffix));
            }
        }
    }
    None
}

/// Run the date regex over `slugged` (which should already use `internal_sep`
/// as its separator) and replace each detected span with
/// `internal_sep + iso + internal_sep` so the surrounding pipeline can
/// collapse it. Returns the substituted string.
#[must_use]
pub fn detect_and_replace(slugged: &str, internal_sep: char, current_year: i32) -> String {
    // First, normalize the candidate substrings so they use `_` as the
    // separator (the format-table key). We do this by replacing
    // `internal_sep` → `_` only inside matched spans - but the simpler
    // approach is to transform the whole input, run detection, then
    // substitute the sentinel back at the call site. Since the slug
    // pipeline already routes through `_`-keyed templates by design
    // (the sentinel is `'\u{E000}'` and we substitute it to `_` before
    // date matching), we accept input that already uses `_`.
    let _ = internal_sep; // unused for now; reserved for future rework
    date_regex()
        .replace_all(slugged, |caps: &regex::Captures<'_>| {
            #[allow(clippy::expect_used)]
            // group(0) always present in a regex match
            let candidate = caps.get(0).expect("regex group 0").as_str();
            match try_date_in_windows(candidate, current_year) {
                Some((prefix, iso, suffix)) => format!("_{prefix}{iso}{suffix}_"),
                None => candidate.to_string(),
            }
        })
        .to_string()
}
