# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-06-18

### BREAKING CHANGES

- `--recursive` / `-r` flag added; `fren rename` now processes only the immediate directory by default. Pass `-r` to restore recursive behavior.

### Added

- `fren reorder` subcommand: moves detected dates to the front of filenames.
