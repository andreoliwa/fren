//! Slugify with optional CamelCase splitting and ISO date detection.
//!
//! Pipeline:
//!
//! 1. NFKC normalize input.
//! 2. (Optional, off by default) Inject `_` at CamelCase boundaries
//!    (`([a-z])([A-Z]+)`). Controlled by `SlugOpts.split_camel`.
//! 3. Inject `_` at "existing time" boundaries (the
//!    `WhatsApp ... at 14.24.19` pattern). Always on - this is part of
//!    date detection, not CamelCase splitting.
//! 4. Slugify via `slug-preserve` using `_` as the internal separator
//!    (so the date-format table - keyed off `_` - matches directly).
//! 5. Run date regex; replace detected spans with their ISO form
//!    wrapped in `_` markers.
//! 6. Apply case mode now that ISO dates are in place.
//! 7. Collapse runs of `_`.
//! 8. Substitute `_` -> user-chosen separator (`SlugOpts.separator`).
//! 9. Trim leading/trailing separators.
//!
//! `_` is used directly as the pipeline separator because the date-format
//! table is keyed off `_`. The user's chosen output separator is currently
//! restricted to non-`_` characters, so there's no collision. If
//! `--separator=_` is ever needed, this module would switch to the PUA
//! sentinel `'\u{E000}'` and rewrite the format table at init.

use crate::date::detect_and_replace;
use crate::SlugOpts;
use chrono::{Datelike, Local};
use regex::Regex;
use slug_preserve::slugify_with_sentinel;
use std::sync::OnceLock;

/// Internal sentinel used by the slugify pipeline. See `slugify::sentinel`
/// for the design rationale; in practice we use `_` directly because the
/// date-format table is keyed off `_`.
const PIPELINE_SEP: char = '_';

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
    let dated = detect_and_replace(&slugged, PIPELINE_SEP, current_year);

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
