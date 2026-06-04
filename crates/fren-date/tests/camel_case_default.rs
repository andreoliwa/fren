#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Tests for the new default behavior: CamelCase splitting is OFF by
//! default. CamelCase / PascalCase tokens pass through untouched.
//!
//! Counterparts in `parity_python_doctests.rs` set
//! `split_camel = true` to match the original Python output.

use fren_date::SlugOpts;
use slug_preserve::CaseMode;

fn rust_default_opts() -> SlugOpts {
    // Defaults: separator '-', case Preserve, split_camel false.
    SlugOpts::default()
}

fn slug(input: &str) -> String {
    fren_date::slugify_camel_iso_with_year(input, &rust_default_opts(), 2024)
}

// Bare CamelCase / PascalCase tokens stay intact.

#[test]
fn whats_app_stays_glued() {
    assert_eq!(slug("WhatsApp"), "WhatsApp");
}

#[test]
fn json_file_stays_glued() {
    assert_eq!(slug("JSONFile"), "JSONFile");
}

#[test]
fn camel_case_token_stays_glued() {
    assert_eq!(slug("CamelCase"), "CamelCase");
}

#[test]
fn pascal_case_token_stays_glued() {
    assert_eq!(slug("PascalCase"), "PascalCase");
}

#[test]
fn lower_camel_token_stays_glued() {
    assert_eq!(slug("myVariable"), "myVariable");
}

// Mixed tokens with spaces still get spaces converted to the separator,
// but the CamelCase tokens themselves remain unsplit.
//
// Note: these tests pass full filenames including extensions because
// the slugify function under test operates on whole strings; the rename
// pipeline upstream splits stem from extension before calling it. Here
// we just want to verify CamelCase boundaries don't split unexpectedly.

#[test]
fn camel_in_phrase_keeps_tokens_intact() {
    assert_eq!(
        slug("CamelCase pascalCase WhatsApp"),
        "CamelCase-pascalCase-WhatsApp"
    );
}

// Date detection still works without camel splitting. The "at" pattern
// for WhatsApp datetime stays on regardless of split_camel.

#[test]
fn whatsapp_at_pattern_still_detected_without_split() {
    assert_eq!(
        slug("WhatsApp Image 2024-01-15 at 12.30.45"),
        "WhatsApp-Image-2024-01-15T12-30-45"
    );
}

#[test]
fn date_detected_alongside_camel_token() {
    assert_eq!(
        slug("WhatsApp Photo 2024-01-15"),
        "WhatsApp-Photo-2024-01-15"
    );
}

// Explicit opt-in via split_camel = true matches Python parity behavior
// (one sanity check; full coverage is in parity_python_doctests.rs).

#[test]
fn explicit_split_camel_works() {
    let opts = SlugOpts {
        separator: '-',
        case: CaseMode::Preserve,
        split_camel: true,
    };
    let out = fren_date::slugify_camel_iso_with_year("WhatsApp Image", &opts, 2024);
    assert_eq!(out, "Whats-App-Image");
}

#[test]
fn explicit_split_camel_with_underscore_separator() {
    let opts = SlugOpts {
        separator: '_',
        case: CaseMode::Preserve,
        split_camel: true,
    };
    let out = fren_date::slugify_camel_iso_with_year("PascalCase", &opts, 2024);
    assert_eq!(out, "Pascal_Case");
}

#[test]
fn iso_t_datetime_without_seconds() {
    // 2026-05-27T1321 -> 2026-05-27T13-21-00
    assert_eq!(
        slug("my-file-2026-05-27T1321"),
        "my-file-2026-05-27T13-21-00"
    );
}

#[test]
fn compact_datetime_without_seconds() {
    // 202605271321 -> 2026-05-27T13-21-00
    assert_eq!(
        slug("my-file-202605271321"),
        "my-file-2026-05-27T13-21-00"
    );
}
