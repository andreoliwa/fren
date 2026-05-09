//! Unicode normalization + character-level transliteration.
//!
//! Uses `unicode-normalization` for NFKC and `deunicode` at the character
//! level (per character, not per string) so the sentinel character is
//! preserved verbatim.

use unicode_normalization::UnicodeNormalization;

/// NFKC-normalize the input string.
#[must_use]
pub fn nfkc(input: &str) -> String {
    input.nfkc().collect()
}

/// Fold non-ASCII characters to ASCII, preserving:
/// - ASCII alphanumerics verbatim
/// - The `keep` sentinel verbatim
/// - All other non-ASCII characters: transliterated via `deunicode_char`
///
/// Characters that `deunicode_char` cannot transliterate are dropped.
#[must_use]
pub fn fold_to_ascii_keep(input: &str, keep: char) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        if c == keep || c.is_ascii() {
            out.push(c);
        } else if let Some(ascii) = deunicode::deunicode_char(c) {
            out.push_str(ascii);
        }
        // else: drop (e.g. exotic scripts deunicode can't handle)
    }
    out
}
