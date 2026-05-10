# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0](https://github.com/andreoliwa/fren/compare/fren-v0.0.0...fren-v0.2.0) - 2026-05-10

### Added

- use renameat2(RENAME_NOREPLACE) on Linux
- stream renames live during --apply
- *(slugify)* make CamelCase splitting opt-in
- portfren to Rust as cargo workspace with library + CLI
- rename files with slugify and date detection

### Other

- add unit tests for case_only and slug_preserve::case
- add proptest invariant tests for executor
- symlinks should not rename the target file
- update README.md
- remove Python, switch pre-commit to Rust
