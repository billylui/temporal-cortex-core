# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-02-16

### Added

- **Truth Engine**: Multi-calendar availability merging with privacy levels (Opaque/Full)
- **Truth Engine**: RRULE expansion with full RFC 5545 support (FREQ, BYDAY, BYSETPOS, COUNT, UNTIL, EXDATE)
- **Truth Engine**: DST-aware expansion — wall-clock times preserved across transitions
- **Truth Engine**: Conflict detection with pairwise overlap and duration calculation
- **Truth Engine**: Free/busy computation with merge and first-fit search
- **Truth Engine**: Leap year handling — `BYMONTHDAY=29` correctly skips non-leap years
- **TOON Core**: JSON-to-TOON encoder with key folding, tabular arrays, and inline arrays
- **TOON Core**: TOON-to-JSON decoder with perfect roundtrip fidelity
- **TOON Core**: Semantic filtering (`filter_and_encode`, `CalendarFilter` presets)
- **TOON CLI**: `toon encode`, `toon decode`, `toon stats` subcommands
- **TOON CLI**: `--filter` and `--filter-preset google` for field stripping
- **WASM**: `@temporal-cortex/toon` npm package (Node.js WASM bindings)
- **WASM**: `@temporal-cortex/truth-engine` npm package (Node.js WASM bindings)
- **Python**: `temporal-cortex-toon` PyPI package (encode, decode, filter_and_encode, expand_rrule, merge_availability)
- **CI**: 4-job pipeline — lint-rust, test-rust, test-wasm, test-python
- **Release**: Automated publishing to crates.io, npm, and PyPI on version tags
- **QA**: 446+ Rust tests, 39+ JS tests, 26 Python tests, ~9,000 property-based tests

[Unreleased]: https://github.com/billylui/temporal-cortex-core/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/billylui/temporal-cortex-core/releases/tag/v0.1.0
