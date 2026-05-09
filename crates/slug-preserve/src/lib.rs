//! `slug-preserve` - case-preserving slugifier.
//!
//! Internal crate for `fren`. Public API is intentionally narrow: a single
//! [`slugify`] entry point plus a [`SlugOpts`] struct controlling separator
//! and case behavior.
//!
//! Unlike the popular `slug` crate (and most other Rust slug crates) which
//! always lowercase, `slug-preserve` exposes a [`CaseMode::Preserve`] mode
//! that keeps the original character case intact, plus `Lower`, `Upper`,
//! `Title`, and `Capitalize` modes for explicit case control.

#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]

mod case;
mod normalize;
mod separator;

pub use case::CaseMode;

/// Options controlling how a string is slugified.
#[derive(Debug, Clone, Copy)]
pub struct SlugOpts {
    /// Output separator character (e.g. `-` or `_`).
    pub separator: char,
    /// How to handle character case.
    pub case: CaseMode,
}

impl Default for SlugOpts {
    fn default() -> Self {
        Self {
            separator: '-',
            case: CaseMode::Preserve,
        }
    }
}

/// Slugify a string using the given options.
#[must_use]
pub fn slugify(input: &str, opts: &SlugOpts) -> String {
    slugify_with_sentinel(input, opts.separator, opts)
}

/// Slugify a string but keep the chosen `sentinel` character as the internal
/// separator throughout, only substituting it for `opts.separator` at the end.
///
/// This entry point is the one `fren` uses: the date-detection pipeline runs
/// over the sentinel-separated form (the date-format table is keyed off the
/// sentinel), and the final pass substitutes sentinel → `opts.separator`.
///
/// Most callers want [`slugify`] instead.
#[must_use]
pub fn slugify_with_sentinel(input: &str, sentinel: char, opts: &SlugOpts) -> String {
    let normalized = normalize::nfkc(input);
    let folded = normalize::fold_to_ascii_keep(&normalized, sentinel);
    let with_sentinels = separator::replace_non_alnum(&folded, sentinel);
    let collapsed = separator::collapse_runs(&with_sentinels, sentinel);
    let cased = case::apply(&collapsed, opts.case);
    let trimmed = cased.trim_matches(sentinel).to_string();
    if sentinel == opts.separator {
        trimmed
    } else {
        trimmed.replace(sentinel, &opts.separator.to_string())
    }
}
