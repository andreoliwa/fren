#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Tests for textual month / 12-hour AM-PM datetime normalization.
//!
//! Covers the ChatGPT-style format `<Month> <D>, <YYYY>, <H>_<MM>_<SS> <AM|PM>`
//! and verifies AM/PM conversion, all 12 months, edge cases (12 AM, 12 PM).

use fren_date::SlugOpts;
use slug_preserve::CaseMode;

fn default_opts() -> SlugOpts {
    SlugOpts {
        separator: '-',
        case: CaseMode::Preserve,
        split_camel: false,
    }
}

fn slug(input: &str) -> String {
    fren_date::slugify_camel_iso(input, &default_opts())
}

// ── ChatGPT canonical format ──────────────────────────────────────────────────

#[test]
fn chatgpt_image_nov_pm() {
    // The motivating example from the spec.
    assert_eq!(
        slug("ChatGPT Image Nov 19, 2025, 11_41_56 AM"),
        "ChatGPT-Image-2025-11-19T11-41-56"
    );
}

// ── AM/PM edge cases ──────────────────────────────────────────────────────────

#[test]
fn noon_12_pm_stays_12() {
    // 12 PM = noon = 12:00 in 24h
    assert_eq!(
        slug("ChatGPT Image Jan 1, 2025, 12_00_00 PM"),
        "ChatGPT-Image-2025-01-01T12-00-00"
    );
}

#[test]
fn midnight_12_am_becomes_00() {
    // 12 AM = midnight = 00:00 in 24h
    assert_eq!(
        slug("ChatGPT Image Jan 1, 2025, 12_00_00 AM"),
        "ChatGPT-Image-2025-01-01T00-00-00"
    );
}

#[test]
fn pm_adds_12_to_hour() {
    // 3 PM = 15h
    assert_eq!(
        slug("ChatGPT Image Mar 5, 2024, 3_07_59 PM"),
        "ChatGPT-Image-2024-03-05T15-07-59"
    );
}

#[test]
fn am_before_noon_unchanged() {
    // 9 AM = 09h
    assert_eq!(
        slug("ChatGPT Image Feb 28, 2023, 9_05_02 AM"),
        "ChatGPT-Image-2023-02-28T09-05-02"
    );
}

// ── All 12 months ─────────────────────────────────────────────────────────────

#[test]
fn all_months() {
    let cases: &[(&str, &str)] = &[
        (
            "ChatGPT Image Jan 15, 2025, 10_00_00 AM",
            "ChatGPT-Image-2025-01-15T10-00-00",
        ),
        (
            "ChatGPT Image Feb 15, 2025, 10_00_00 AM",
            "ChatGPT-Image-2025-02-15T10-00-00",
        ),
        (
            "ChatGPT Image Mar 15, 2025, 10_00_00 AM",
            "ChatGPT-Image-2025-03-15T10-00-00",
        ),
        (
            "ChatGPT Image Apr 15, 2025, 10_00_00 AM",
            "ChatGPT-Image-2025-04-15T10-00-00",
        ),
        (
            "ChatGPT Image May 15, 2025, 10_00_00 AM",
            "ChatGPT-Image-2025-05-15T10-00-00",
        ),
        (
            "ChatGPT Image Jun 15, 2025, 10_00_00 AM",
            "ChatGPT-Image-2025-06-15T10-00-00",
        ),
        (
            "ChatGPT Image Jul 15, 2025, 10_00_00 AM",
            "ChatGPT-Image-2025-07-15T10-00-00",
        ),
        (
            "ChatGPT Image Aug 15, 2025, 10_00_00 AM",
            "ChatGPT-Image-2025-08-15T10-00-00",
        ),
        (
            "ChatGPT Image Sep 15, 2025, 10_00_00 AM",
            "ChatGPT-Image-2025-09-15T10-00-00",
        ),
        (
            "ChatGPT Image Oct 15, 2025, 10_00_00 AM",
            "ChatGPT-Image-2025-10-15T10-00-00",
        ),
        (
            "ChatGPT Image Nov 15, 2025, 10_00_00 AM",
            "ChatGPT-Image-2025-11-15T10-00-00",
        ),
        (
            "ChatGPT Image Dec 15, 2025, 10_00_00 AM",
            "ChatGPT-Image-2025-12-15T10-00-00",
        ),
    ];
    for (input, expected) in cases {
        assert_eq!(slug(input), *expected, "failed for input: {input}");
    }
}

// ── Case-insensitive month names ──────────────────────────────────────────────

#[test]
fn lowercase_month_name() {
    assert_eq!(
        slug("ChatGPT Image nov 19, 2025, 11_41_56 AM"),
        "ChatGPT-Image-2025-11-19T11-41-56"
    );
}

#[test]
fn uppercase_month_name() {
    assert_eq!(
        slug("ChatGPT Image NOV 19, 2025, 11_41_56 AM"),
        "ChatGPT-Image-2025-11-19T11-41-56"
    );
}

// ── Case-insensitive AM/PM ────────────────────────────────────────────────────

#[test]
fn lowercase_am() {
    assert_eq!(
        slug("ChatGPT Image Nov 19, 2025, 11_41_56 am"),
        "ChatGPT-Image-2025-11-19T11-41-56"
    );
}

#[test]
fn lowercase_pm() {
    assert_eq!(
        slug("ChatGPT Image Nov 19, 2025, 3_07_59 pm"),
        "ChatGPT-Image-2025-11-19T15-07-59"
    );
}

#[test]
fn mixed_case_am() {
    assert_eq!(
        slug("ChatGPT Image Nov 19, 2025, 11_41_56 Am"),
        "ChatGPT-Image-2025-11-19T11-41-56"
    );
}

// ── Different time separators ─────────────────────────────────────────────────

#[test]
fn colon_time_separator() {
    assert_eq!(
        slug("ChatGPT Image Nov 19, 2025, 11:41:56 AM"),
        "ChatGPT-Image-2025-11-19T11-41-56"
    );
}
