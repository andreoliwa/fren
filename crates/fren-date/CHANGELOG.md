# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0](https://github.com/andreoliwa/fren/compare/fren-date-v0.2.2...fren-date-v0.3.0) - 2026-06-27

### Added

- *(plan)* add plan_reorder and plan_reorder_with_year
- *(slugify)* add slugify_camel_iso_detect returning DetectedDate
- *(parser)* add detect_and_replace_with_span and DetectedDate.iso_string

### Fixed

- *(plan)* slugify date-less files in plan_reorder_with_year
- *(plan_types)* mark DetectedDate as non_exhaustive
- *(plan)* include file roots in rename and reorder plans

### Other

- *(reorder)* apply rustfmt
- *(parser)* remove dead internal_sep param and collapse parallel date-scan impls
- *(reorder)* add CLI and library integration tests, update docs
- *(reorder)* add failing tests for plan_reorder_with_year
- *(recursive)* add shallow-default and deep-walk tests

## [0.2.2](https://github.com/andreoliwa/fren/compare/fren-date-v0.2.1...fren-date-v0.2.2) - 2026-06-16

### Added

- _(slugify)_ normalize textual month/AM-PM datetimes before numeric pipeline
- _(conflict)_ rewrite conflict resolution to be policy-aware
- _(conflict)_ add numbered_name helper with unit tests
- _(date)_ recognize unpadded and digit-surrounded dates
- _(date)_ parse datetime when followed by a trailing sequence number
- _(date)_ recognize HHmm datetimes and ISO T separator

### Fixed

- _(conflict)_ prevent cross-group copy-number collision and default policy to Number

### Other

- _(slugify)_ add tests for textual month, AM/PM normalization, and case variants
- _(deps)_ add pure-rust-locales, upgrade anstream 0.6 to 1
- _(proptest)_ switch invariants to Number policy and fix invariant2 for within-batch collisions
- _(date)_ apply rustfmt to pre-existing format issues
- _(proptest)_ record case-only rename regression seed
