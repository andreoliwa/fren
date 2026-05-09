// Test code legitimately uses unwrap/expect/panic for assertions and
// fixture setup; the library-level lints are too strict for tests.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Parity tests: every doctest from the original Python
//! `slugify_camel_iso` is reproduced here as a Rust unit test.
//!
//! These tests use `slugify_camel_iso_with_year` so the "current year"
//! is pinned to `2019`, matching the doctests' implicit assumption that
//! 2-digit years like `27` parse as 2027 (+10-year rule) and `75` parses
//! as 1975.
//!
//! Tests use `SlugOpts { separator: '_', case: Capitalize }` to match the
//! original Python output exactly.

use fren::SlugOpts;
use slug_preserve::CaseMode;

fn legacy_python_opts() -> SlugOpts {
    SlugOpts {
        separator: '_',
        case: CaseMode::Capitalize,
        // Python always split CamelCase. Match that behavior in the
        // parity tests; the new default in Rust is `false` (see the
        // separate test file `camel_case_default.rs`).
        split_camel: true,
    }
}

fn slug(input: &str) -> String {
    fren::slugify_camel_iso_with_year(input, &legacy_python_opts(), 2019)
}

// One test per Python doctest line, named after the input shape.

#[test]

fn doctest_basic_iso_year() {
    assert_eq!(
        slug("some name Here 2017_12_30"),
        "Some_Name_Here_2017-12-30"
    );
}

#[test]

fn doctest_dont_pay_bill() {
    assert_eq!(
        slug("DONT_PAY_this-bill-10-05-2015"),
        "Dont_Pay_This_Bill_2015-05-10"
    );
}

#[test]

fn doctest_normal_date_no_dashes() {
    assert_eq!(
        slug("normal DATE 01012019 with no DASHES"),
        "Normal_Date_2019-01-01_With_No_Dashes"
    );
}

#[test]

fn doctest_normal_date_underscores() {
    assert_eq!(
        slug("normal DATE 23_05_2019 with underscores"),
        "Normal_Date_2019-05-23_With_Underscores"
    );
}

#[test]

fn doctest_inverted_date_no_dashes() {
    assert_eq!(
        slug("inverted DATE 20191020 with no DASHES"),
        "Inverted_Date_2019-10-20_With_No_Dashes"
    );
}

#[test]

fn doctest_unicode_scream() {
    assert_eq!(
        slug("blablabla-SCREAM LOUD AGAIN - XXX UTILIZAÇÃO 27.11.17"),
        "Blablabla_Scream_Loud_Again_Xxx_Utilizacao_2017-11-27"
    );
}

#[test]

fn doctest_ata_aug_2_digit_year() {
    assert_eq!(
        slug("something-614 ATA AUG IN 25-04-17"),
        "Something_614_Ata_Aug_In_2017-04-25"
    );
}

#[test]

fn doctest_inverted_datetime() {
    assert_eq!(
        slug("inverted 2017_12_30_10_44_56 bla"),
        "Inverted_2017-12-30T10-44-56_Bla"
    );
}

#[test]

fn doctest_normal_datetime() {
    assert_eq!(
        slug("normal 30.12.2017_10_44_56 bla"),
        "Normal_2017-12-30T10-44-56_Bla"
    );
}

#[test]

fn doctest_no_day_inverted() {
    assert_eq!(slug(" no day inverted 1975 08 "), "No_Day_Inverted_1975-08");
}

#[test]

fn doctest_no_day_normal() {
    assert_eq!(slug(" no day normal 08 1975 "), "No_Day_Normal_1975-08");
}

#[test]

fn doctest_camel_pascal_json_whatsapp() {
    assert_eq!(
        slug(" CamelCase pascalCase JSONfile WhatsApp"),
        "Camel_Case_Pascal_Case_Jsonfile_Whats_App"
    );
}

#[test]

fn doctest_keep_formatted_iso_datetime() {
    assert_eq!(
        slug(" 2019-08-22T16-01-22 keep formatted times "),
        "2019-08-22T16-01-22_Keep_Formatted_Times"
    );
}

#[test]

fn doctest_whatsapp_ptt_at() {
    assert_eq!(
        slug("WhatsApp Ptt 2019-08-21 at 14.24.19"),
        "Whats_App_Ptt_2019-08-21T14-24-19"
    );
}

#[test]

fn doctest_whatsapp_image_already_slugged() {
    assert_eq!(
        slug("Whats_App_Image_2019-08-23_At_12_34_55 fix times on whatsapp files"),
        "Whats_App_Image_2019-08-23T12-34-55_Fix_Times_On_Whatsapp_Files"
    );
}

#[test]

fn doctest_whatsapp_zip() {
    assert_eq!(
        slug("Whats_App_Zip_2019-08-23_At_13_23.36"),
        "Whats_App_Zip_2019-08-23T13-23-36"
    );
}

#[test]

fn doctest_fwd_consulta() {
    assert_eq!(slug("fwdConsultaCognicao"), "Fwd_Consulta_Cognicao");
}

#[test]

fn doctest_unicode_bancarios() {
    assert_eq!(
        slug("bla Bancários - Atenção ble"),
        "Bla_Bancarios_Atencao_Ble"
    );
}

#[test]

fn doctest_two_digit_years_pair() {
    assert_eq!(
        slug(" 240819 human day month year 290875 "),
        "2019-08-24_Human_Day_Month_Year_1975-08-29"
    );
}

#[test]

fn doctest_iso_glued_to_words() {
    assert_eq!(
        slug("2019-08-23T12-48-26words with numbers"),
        "2019-08-23T12-48-26_Words_With_Numbers"
    );
}

#[test]

fn doctest_some_yyyymmdd_underscore_hhmmss() {
    assert_eq!(
        slug("some 20180726_224001 thing"),
        "Some_2018-07-26T22-40-01_Thing"
    );
}

#[test]

fn doctest_glued_ddmmyyyy() {
    assert_eq!(slug("glued14092019"), "Glued_2019-09-14");
}

#[test]

fn doctest_glued_iso_datetime() {
    assert_eq!(
        slug("glued2019-08-23T12-48-26"),
        "Glued_2019-08-23T12-48-26"
    );
}

#[test]

fn doctest_yeah_yyyy_mm() {
    assert_eq!(slug("yeah-1975-08"), "Yeah_1975-08");
}

#[test]

fn doctest_visa_yyyy_mm() {
    assert_eq!(slug("xxx visa-2013-07 yyy"), "Xxx_Visa_2013-07_Yyy");
}

#[test]

fn doctest_date_without_seconds() {
    assert_eq!(
        slug("date without seconds 101020191830 "),
        "Date_Without_Seconds_2019-10-10T18-30-00"
    );
}

#[test]

fn doctest_p2p_b2b_1on1() {
    assert_eq!(
        slug(" p2p b2b 1on1 P2P B2B 1ON1 "),
        "P2p_B2b_1on1_P2p_B2b_1on1"
    );
}

// New format: YYYY_MM_DD_HH_mm (minute precision with separators, no seconds).
// Output should be DateTime with zero seconds appended.
#[test]
fn datetime_iso_minute_only_dashed() {
    assert_eq!(
        slug("note 2026-05-03-18-57 minute only"),
        "Note_2026-05-03T18-57-00_Minute_Only"
    );
}

#[test]
fn datetime_iso_minute_only_human() {
    assert_eq!(
        slug("scan 03-05-2026-18-57 human minute only"),
        "Scan_2026-05-03T18-57-00_Human_Minute_Only"
    );
}

// Smoke test that runs even without Capitalize: just verify the function
// produces *something* (no panic, no empty string for non-empty input).
#[test]
fn smoke_function_runs_on_basic_input() {
    let out =
        fren::slugify_camel_iso_with_year("Hello World 2024-01-15.txt", &SlugOpts::default(), 2024);
    assert!(!out.is_empty(), "output should not be empty: got {out:?}");
}
