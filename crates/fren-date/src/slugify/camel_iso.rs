//! Slugify with optional CamelCase splitting and ISO date detection.
//!
//! Pipeline:
//!
//! 1. Normalize textual month/AM-PM datetime patterns (e.g. `Nov 19, 2025,
//!    11_41_56 AM`) into fully-numeric form before any other step.
//! 2. NFKC normalize input.
//! 3. (Optional, off by default) Inject `_` at CamelCase boundaries
//!    (`([a-z])([A-Z]+)`). Controlled by `SlugOpts.split_camel`.
//! 4. Inject `_` at "existing time" boundaries (the
//!    `WhatsApp ... at 14.24.19` pattern). Always on - this is part of
//!    date detection, not CamelCase splitting.
//! 5. Slugify via `slug-preserve` using `_` as the internal separator
//!    (so the date-format table - keyed off `_` - matches directly).
//! 6. Run date regex; replace detected spans with their ISO form
//!    wrapped in `_` markers.
//! 7. Apply case mode now that ISO dates are in place.
//! 8. Collapse runs of `_`.
//! 9. Substitute `_` -> user-chosen separator (`SlugOpts.separator`).
//! 10. Trim leading/trailing separators.
//!
//! `_` is used directly as the pipeline separator because the date-format
//! table is keyed off `_`. The user's chosen output separator is currently
//! restricted to non-`_` characters, so there's no collision. If
//! `--separator=_` is ever needed, this module would switch to the PUA
//! sentinel `'\u{E000}'` and rewrite the format table at init.

use crate::date::{detect_and_replace, detect_and_replace_with_span};
use crate::SlugOpts;
use chrono::{Datelike, Local};
use regex::Regex;
use slug_preserve::slugify_with_sentinel;
use std::sync::OnceLock;

/// Internal sentinel used by the slugify pipeline. See `slugify::sentinel`
/// for the design rationale; in practice we use `_` directly because the
/// date-format table is keyed off `_`.
const PIPELINE_SEP: char = '_';

/// Locales whose abbreviated month names are recognized during textual datetime
/// normalization. Adding a language here is sufficient to support it - the
/// month alternation in the regex and the lookup table are both built from
/// these locale ABMON arrays at init time.
///
/// Locale data comes from `pure-rust-locales` (GNU libc locale database).
const RECOGNIZED_LOCALES: &[&[&str]] = &[
    pure_rust_locales::en_US::LC_TIME::ABMON,
    // Add more locales here, e.g.:
    // pure_rust_locales::pt_BR::LC_TIME::ABMON,
    // pure_rust_locales::de_DE::LC_TIME::ABMON,
];

/// Build a deduplicated, case-folded map from abbreviated month name to
/// 1-based month number, drawing from all `RECOGNIZED_LOCALES`.
fn month_map() -> &'static Vec<(String, u32)> {
    static MAP: OnceLock<Vec<(String, u32)>> = OnceLock::new();
    MAP.get_or_init(|| {
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut map = Vec::new();
        for abmon in RECOGNIZED_LOCALES {
            for (i, &name) in abmon.iter().enumerate() {
                let key = name.to_ascii_lowercase();
                if seen.insert(key.clone()) {
                    map.push((key, i as u32 + 1));
                }
            }
        }
        map
    })
}

fn month_number(name: &str) -> Option<u32> {
    let key = name.to_ascii_lowercase();
    month_map().iter().find(|(k, _)| k == &key).map(|(_, n)| *n)
}

/// Regex matching a textual datetime: `<MonthName> <D|DD>, <YYYY>,
/// <H|HH>[sep]<MM>[sep]<SS> <AM|PM>` where month names are drawn from all
/// configured locales.
///
/// Examples:
///   `Nov 19, 2025, 11_41_56 AM`
///   `Jan 3, 2026, 9:05:02 PM`
fn re_textual_datetime() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // Build the month alternation from the same locale data used by
        // month_number(), so the regex and the lookup are always in sync.
        let mut names: Vec<String> = month_map()
            .iter()
            .map(|(k, _)| regex::escape(k))
            .collect();
        // Longer names first to avoid prefix shadowing in alternation.
        names.sort_by_key(|a| std::cmp::Reverse(a.len()));
        let alt = names.join("|");
        let pattern = format!(
            r"(?i)\b({alt})[.,\s]+(\d{{1,2}})[,\s]+(\d{{4}})[,\s]+(\d{{1,2}})[_:\.\-](\d{{2}})[_:\.\-](\d{{2}})\s*(AM|PM)\b"
        );
        #[allow(clippy::expect_used)]
        Regex::new(&pattern).expect("textual-datetime regex compiles")
    })
}

/// Replace textual month / 12-hour-clock datetime spans with numeric form.
///
/// `Nov 19, 2025, 11_41_56 AM` -> `19_11_2025_11_41_56`
/// `Jan 3, 2026, 9:05:02 PM`   -> `03_01_2026_21_05_02`
fn normalize_textual_datetime(input: &str) -> String {
    re_textual_datetime()
        .replace_all(input, |caps: &regex::Captures<'_>| {
            #[allow(clippy::expect_used)]
            let month_str = caps.get(1).expect("group 1").as_str();
            #[allow(clippy::expect_used)]
            let day: u32 = caps.get(2).expect("group 2").as_str().parse().unwrap_or(1);
            #[allow(clippy::expect_used)]
            let year: u32 = caps.get(3).expect("group 3").as_str().parse().unwrap_or(0);
            #[allow(clippy::expect_used)]
            let mut hour: u32 = caps.get(4).expect("group 4").as_str().parse().unwrap_or(0);
            #[allow(clippy::expect_used)]
            let minute: u32 = caps.get(5).expect("group 5").as_str().parse().unwrap_or(0);
            #[allow(clippy::expect_used)]
            let second: u32 = caps.get(6).expect("group 6").as_str().parse().unwrap_or(0);
            #[allow(clippy::expect_used)]
            let meridiem = caps.get(7).expect("group 7").as_str();

            hour = match meridiem.to_ascii_uppercase().as_str() {
                "AM" => {
                    if hour == 12 {
                        0
                    } else {
                        hour
                    }
                }
                _ => {
                    if hour == 12 {
                        12
                    } else {
                        hour + 12
                    }
                }
            };

            let month = month_number(month_str).unwrap_or(1);
            format!("{day:02}_{month:02}_{year}_{hour:02}_{minute:02}_{second:02}")
        })
        .into_owned()
}

fn re_camelcase() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        #[allow(clippy::expect_used)]
        Regex::new(r"([a-z])([A-Z]+)").expect("static camelcase regex compiles")
    })
}

fn re_existing_time() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // Mirrors Python: r"(-[0-9]{2})[ _]?[Aa]?[Tt][ _]?([0-9]{2}[-._])"
        #[allow(clippy::expect_used)]
        Regex::new(r"(-[0-9]{2})[ _]?[Aa]?[Tt][ _]?([0-9]{2}[\-._])")
            .expect("static existing-time regex compiles")
    })
}

fn re_iso_t_sep() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // Normalize ISO 8601 `T` separator between date and time digits into
        // the pipeline separator so the date parser can match across it.
        // Matches only when `T` sits between two digits to avoid clobbering
        // unrelated uppercase T tokens.
        #[allow(clippy::expect_used)]
        Regex::new(r"(\d)T(\d)").expect("static iso-T-sep regex compiles")
    })
}

fn re_multiple_underscore() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        #[allow(clippy::expect_used)]
        Regex::new(r"_+").expect("static underscore-collapse regex compiles")
    })
}

/// Slugify a string with CamelCase splitting, ISO date detection, and the
/// user's chosen output separator/case.
///
/// This is the orchestrator the file-rename and reorder paths invoke per
/// item.
#[must_use]
pub fn slugify_camel_iso(input: &str, opts: &SlugOpts) -> String {
    let current_year = Local::now().year();
    slugify_camel_iso_with_year(input, opts, current_year)
}

/// Variant exposing the "current year" so tests can pin time.
#[must_use]
pub fn slugify_camel_iso_with_year(input: &str, opts: &SlugOpts, current_year: i32) -> String {
    // Step 0: convert textual month / 12-hour datetime to numeric form
    // (e.g. `Nov 19, 2025, 11_41_56 AM` -> `19_11_2025_11_41_56`) so the
    // downstream numeric date parser can handle it.
    let normalized = normalize_textual_datetime(input);
    let input = normalized.as_str();

    // Step 1+2+3: NFKC + inject separators.
    // We do NFKC inside slug-preserve, but need to do the regex injects
    // here first (Python does NFKC then both regex injects, then slugify).
    // Order: NFKC → existing-time inject → camelcase inject → slugify.
    let nfkc: String = unicode_normalization::UnicodeNormalization::nfkc(input).collect();
    let with_time = re_existing_time()
        .replace_all(&nfkc, |c: &regex::Captures<'_>| {
            #[allow(clippy::expect_used)]
            let g1 = c.get(1).expect("regex group 1").as_str();
            #[allow(clippy::expect_used)]
            let g2 = c.get(2).expect("regex group 2").as_str();
            format!("{g1}_{g2}")
        })
        .into_owned();
    let with_camel = if opts.split_camel {
        re_camelcase()
            .replace_all(&with_time, |c: &regex::Captures<'_>| {
                #[allow(clippy::expect_used)]
                let g1 = c.get(1).expect("regex group 1").as_str();
                #[allow(clippy::expect_used)]
                let g2 = c.get(2).expect("regex group 2").as_str();
                format!("{g1}_{g2}")
            })
            .into_owned()
    } else {
        with_time
    };

    // Step 4: slugify with PIPELINE_SEP as sentinel. Always Preserve case
    // here - case transformation happens after date detection so that
    // ISO date substrings emitted by detect_and_replace get the correct
    // case treatment (Python applies its `_[a-z] -> _X` post-pass after
    // dates are inserted).
    let pipeline_opts = SlugOpts {
        separator: PIPELINE_SEP,
        case: slug_preserve::CaseMode::Preserve,
        split_camel: opts.split_camel,
    };
    let slugged = slugify_with_sentinel(&with_camel, PIPELINE_SEP, &pipeline_opts);

    // Step 4b: normalize ISO 8601 `T` between digits into `_` so the date
    // parser can treat `2026_05_27T1321` as a single candidate span.
    let slugged = re_iso_t_sep()
        .replace_all(&slugged, |c: &regex::Captures<'_>| {
            #[allow(clippy::expect_used)]
            let g1 = c.get(1).expect("regex group 1").as_str();
            #[allow(clippy::expect_used)]
            let g2 = c.get(2).expect("regex group 2").as_str();
            format!("{g1}_{g2}")
        })
        .into_owned();

    // Step 5: detect dates and substitute their spans with ISO output
    // wrapped in `_` markers.
    let dated = detect_and_replace(&slugged, current_year);

    // Step 6: apply case mode now that dates are in place.
    let cased = slug_preserve_apply_case(&dated, opts.case);

    // Step 7: collapse runs of `_`.
    let collapsed = re_multiple_underscore()
        .replace_all(&cased, "_")
        .into_owned();

    // Step 8: substitute pipeline sep → user separator.
    // Step 9: trim trailing/leading separator chars.
    let final_str = if PIPELINE_SEP == opts.separator {
        collapsed
    } else {
        collapsed.replace(PIPELINE_SEP, &opts.separator.to_string())
    };
    final_str.trim_matches(opts.separator).to_string()
}

/// Variant of [`slugify_camel_iso_with_year`] that also returns the first
/// detected date's metadata. Used by the reorder planner.
///
/// Returns `(slugified_name, Option<DetectedDate>)`. The `byte_span` inside
/// `DetectedDate` is relative to the **intermediate** string produced after
/// step 5 (post-NFKC, post-slug, at the point of date substitution,
/// pre-case-transform). Callers MUST NOT slice the final slug by `byte_span`
/// directly because case transforms between step 5 and the final output may
/// shift byte offsets. Instead, use `detected.iso_string` to locate the date
/// in the final slug by string search (e.g. `final_slug.find(&detected.iso_string)`).
///
/// ISO date strings consist of ASCII digits and hyphens, which are
/// case-invariant. The `iso_string` value returned here remains valid for
/// string-search in the final output regardless of which case mode is active.
#[must_use]
pub fn slugify_camel_iso_detect(
    input: &str,
    opts: &SlugOpts,
    current_year: i32,
) -> (String, Option<crate::DetectedDate>) {
    // Step 0: convert textual month / 12-hour datetime to numeric form
    let normalized = normalize_textual_datetime(input);
    let input = normalized.as_str();

    // Steps 1-3: NFKC + inject separators (same as slugify_camel_iso_with_year)
    let nfkc: String = unicode_normalization::UnicodeNormalization::nfkc(input).collect();
    let with_time = re_existing_time()
        .replace_all(&nfkc, |c: &regex::Captures<'_>| {
            #[allow(clippy::expect_used)]
            let g1 = c.get(1).expect("regex group 1").as_str();
            #[allow(clippy::expect_used)]
            let g2 = c.get(2).expect("regex group 2").as_str();
            format!("{g1}_{g2}")
        })
        .into_owned();
    let with_camel = if opts.split_camel {
        re_camelcase()
            .replace_all(&with_time, |c: &regex::Captures<'_>| {
                #[allow(clippy::expect_used)]
                let g1 = c.get(1).expect("regex group 1").as_str();
                #[allow(clippy::expect_used)]
                let g2 = c.get(2).expect("regex group 2").as_str();
                format!("{g1}_{g2}")
            })
            .into_owned()
    } else {
        with_time
    };

    // Step 4: slugify with PIPELINE_SEP as sentinel
    let pipeline_opts = SlugOpts {
        separator: PIPELINE_SEP,
        case: slug_preserve::CaseMode::Preserve,
        split_camel: opts.split_camel,
    };
    let slugged = slugify_with_sentinel(&with_camel, PIPELINE_SEP, &pipeline_opts);

    // Step 4b: normalize ISO 8601 `T` between digits into `_`
    let slugged = re_iso_t_sep()
        .replace_all(&slugged, |c: &regex::Captures<'_>| {
            #[allow(clippy::expect_used)]
            let g1 = c.get(1).expect("regex group 1").as_str();
            #[allow(clippy::expect_used)]
            let g2 = c.get(2).expect("regex group 2").as_str();
            format!("{g1}_{g2}")
        })
        .into_owned();

    // Step 5: detect dates and substitute spans, also returning the first
    // detected date's metadata.
    let (dated, detected) = detect_and_replace_with_span(&slugged, current_year);

    // Steps 6-9: apply case, collapse underscores, substitute separator, trim
    let cased = slug_preserve_apply_case(&dated, opts.case);
    let collapsed = re_multiple_underscore()
        .replace_all(&cased, "_")
        .into_owned();
    let final_str = if PIPELINE_SEP == opts.separator {
        collapsed
    } else {
        collapsed.replace(PIPELINE_SEP, &opts.separator.to_string())
    };
    let result = final_str.trim_matches(opts.separator).to_string();
    (result, detected)
}

fn slug_preserve_apply_case(input: &str, mode: slug_preserve::CaseMode) -> String {
    // We re-export only what's exposed from slug_preserve; case::apply is
    // pub(crate) there. We replicate the call via the SlugOpts entry.
    // Simplest path: build a one-off SlugOpts and run slugify with the
    // chosen mode but this would re-tokenize. Instead: do the case work
    // inline.
    use slug_preserve::CaseMode;
    match mode {
        CaseMode::Preserve => input.to_string(),
        CaseMode::Lower => input.to_lowercase(),
        CaseMode::Upper => input.to_uppercase(),
        CaseMode::Title | CaseMode::Capitalize => title_case_after_alnum_boundary(input),
    }
}

/// Title-case for the post-date pipeline.
///
/// Mirrors Python's effective behavior:
///
/// 1. `python-slugify` lowercases everything.
/// 2. `.capitalize()` uppercases just the very first character.
/// 3. After date substitution, regex `_[a-z] → _X` uppercases any
///    lowercase letter immediately after `_`.
///
/// Net effect: every letter is lowercase, except the first char of the
/// whole string and the first char of each `_`-delimited word. ISO
/// datetime output's `T` stays uppercase because by the time the regex
/// post-pass runs, `T` is already uppercase (chrono emits it uppercase),
/// and the regex only **adds** uppercase, never removes it.
///
/// We replicate the Python net effect by lowercasing first, then
/// uppercasing the first char and any char after `_`. This means the `T`
/// in `T12-48-26` gets temporarily lowercased to `t` then **stays
/// lowercase** in our output. To match Python, we instead exempt
/// uppercase-A-Z letters that originated from the date substitution.
/// Since date substitution emits ISO with a fixed shape, the simplest
/// approach is to detect the digit-letter-digit pattern (`5T1`, `0T0`,
/// etc.) and preserve it. Or simpler: after lowercasing, restore `T`
/// when it sits between digits.
fn title_case_after_alnum_boundary(input: &str) -> String {
    // Step 1: lowercase the whole string.
    let lowered = input.to_lowercase();
    let bytes = lowered.as_bytes();

    // Step 2: walk and uppercase: (a) first char, (b) char after `_`,
    // (c) `t` between two digits (ISO datetime separator).
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    for (i, &b) in bytes.iter().enumerate() {
        let mut ch = b;
        if ch.is_ascii_lowercase() {
            let after_underscore = i > 0 && bytes[i - 1] == b'_';
            let at_start = i == 0;
            let between_digits = ch == b't'
                && i > 0
                && i + 1 < bytes.len()
                && bytes[i - 1].is_ascii_digit()
                && bytes[i + 1].is_ascii_digit();
            if at_start || after_underscore || between_digits {
                ch = ch.to_ascii_uppercase();
            }
        }
        out.push(ch);
    }
    // Safe: we only modified ASCII bytes, original was UTF-8.
    #[allow(clippy::expect_used)]
    String::from_utf8(out).expect("ASCII-only mutations preserve UTF-8")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn detect_variant_returns_detected_date() {
        // Stem only (no extension), space-separated, date is DD-MM-YYYY
        let input = "IMG 01-12-2025 something";
        let (slug, maybe) = slugify_camel_iso_detect(input, &SlugOpts::default(), 2024);
        let detected = maybe.expect("should detect a date");
        assert_eq!(detected.iso_string, "2025-12-01");
        assert!(
            slug.contains("2025-12-01"),
            "slug should contain iso date, got: {slug}"
        );
    }

    #[test]
    fn detect_variant_returns_none_for_no_date() {
        let input = "some file name";
        let (slug, maybe) = slugify_camel_iso_detect(input, &SlugOpts::default(), 2024);
        assert!(maybe.is_none());
        assert_eq!(slug, "some-file-name");
    }

    #[test]
    fn detect_variant_matches_with_year_for_same_input() {
        let inputs = [
            "IMG_01_12_2025_something",
            "Meeting 2024-01-15 notes",
            "plain text no date",
        ];
        let opts = SlugOpts::default();
        let year = 2024;
        for input in inputs {
            let (slug_detect, _) = slugify_camel_iso_detect(input, &opts, year);
            let slug_plain = slugify_camel_iso_with_year(input, &opts, year);
            assert_eq!(
                slug_detect, slug_plain,
                "slugify_camel_iso_detect slug must match slugify_camel_iso_with_year for input: {input}"
            );
        }
    }
}
