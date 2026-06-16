# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.2](https://github.com/andreoliwa/fren/compare/fren-date-v0.2.1...fren-date-v0.2.2) - 2026-06-16

### Added

- *(slugify)* normalize textual month/AM-PM datetimes before numeric pipeline
- *(conflict)* rewrite conflict resolution to be policy-aware
- *(conflict)* add numbered_name helper with unit tests
- *(date)* recognize unpadded and digit-surrounded dates
- *(date)* parse datetime when followed by a trailing sequence number
- *(date)* recognize HHmm datetimes and ISO T separator

### Fixed

- *(conflict)* prevent cross-group copy-number collision and default policy to Number

### Other

- *(slugify)* add tests for textual month, AM/PM normalization, and case variants
- *(deps)* add pure-rust-locales, upgrade anstream 0.6 to 1
- *(proptest)* switch invariants to Number policy and fix invariant2 for within-batch collisions
- *(date)* apply rustfmt to pre-existing format issues
- *(proptest)* record case-only rename regression seed
