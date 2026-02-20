# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-02-20

### Added
- **Truth Engine**: New `temporal` module with 4 pure computation functions:
  - `convert_timezone` — DST-aware timezone conversion with offset and DST status
  - `compute_duration` — Duration between two timestamps (days/hours/minutes/seconds breakdown)
  - `adjust_timestamp` — DST-aware timestamp adjustment (compound duration format: `+1d2h30m`)
  - `resolve_relative` — Human expression parser (`"next Tuesday at 2pm"`, `"tomorrow morning"`, `"+2h"`, 60+ patterns across 9 categories) to RFC 3339
- **WASM/JS**: 4 new bindings for temporal functions (`convertTimezone`, `computeDuration`, `adjustTimestamp`, `resolveRelative`)
- **Python**: 4 new functions (`convert_timezone`, `compute_duration`, `adjust_timestamp`, `resolve_relative`)
- **Python/JS**: In-library contextual hint when `merge_availability()` is called with 3+ event streams, linking to Platform early access (suppressable via `TEMPORAL_CORTEX_QUIET` env var)

### Changed
- **truth-engine**: Updated crate description and keywords to reflect temporal computation capabilities
- **Docs**: Added "Temporal Computation" section to Core README with code examples
- **Docs**: Rewrote "Going to Production?" README section with pain-led copy and pricing table
- **Python**: Restructured package to maturin mixed layout (Rust `_native` extension + Python wrapper in `__init__.py`)

## [0.1.1] - 2026-02-18

### Changed
- **Dependencies**: Upgraded rrule 0.13→0.14, PyO3 0.23→0.28, criterion 0.5→0.8, vitest 3→4, @types/node 20→25, clap 4.5.58→4.5.59
- **CI**: Upgraded GitHub Actions — checkout v4→v6, setup-python v5→v6, setup-node v4→v6, cache v4→v5
- **Security**: Removed RUSTSEC-2025-0020 advisory ignore (fixed in PyO3 0.28)
- **Docs**: Fixed stale rrule version references in Truth Engine README and source comments
- **Config**: Restored dependabot.yml for automated dependency updates

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

[Unreleased]: https://github.com/billylui/temporal-cortex-core/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/billylui/temporal-cortex-core/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/billylui/temporal-cortex-core/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/billylui/temporal-cortex-core/releases/tag/v0.1.0
