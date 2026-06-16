# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.2](https://github.com/andreoliwa/fren/compare/fren-date-cli-v0.2.1...fren-date-cli-v0.2.2) - 2026-06-16

### Added

- _(rename)_ add --on-conflict flag defaulting to number

### Fixed

- _(rename)_ exhaustive match on --on-conflict, no wildcard fallthrough

### Other

- _(deps)_ add pure-rust-locales, upgrade anstream 0.6 to 1
- _(on_conflict)_ apply rustfmt
- _(cli)_ add integration tests for --on-conflict behavior
