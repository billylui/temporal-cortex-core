# Temporal Cortex Core

[![CI](https://github.com/billylui/temporal-cortex-core/actions/workflows/ci.yml/badge.svg)](https://github.com/billylui/temporal-cortex-core/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/truth-engine.svg)](https://crates.io/crates/truth-engine)
[![npm](https://img.shields.io/npm/v/@temporal-cortex/truth-engine.svg)](https://www.npmjs.com/package/@temporal-cortex/truth-engine)
[![PyPI](https://img.shields.io/pypi/v/temporal-cortex-toon.svg)](https://pypi.org/project/temporal-cortex-toon/)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

Deterministic calendar computation for AI agents. Merge availability across calendars, expand recurrence rules, detect conflicts, and compress calendar data — all without LLM inference.

## The Problem

AI agents scheduling on behalf of humans face a fragmented calendar landscape. A person's availability is split across Google Calendar, Outlook, and iCloud — but no single agent can see all of them. The result: double-bookings, missed conflicts, and date hallucinations.

**Truth Engine** solves this by providing a pure computation layer that merges multiple calendar event streams into a single, privacy-preserving availability view. **TOON** complements it with 50%+ token reduction for calendar payloads.

| Library | What it does |
|---------|-------------|
| **Truth Engine** | Multi-calendar availability merging, RRULE expansion, conflict detection, free/busy computation |
| **TOON** | Token-Oriented Object Notation — compact serialization achieving 50%+ token reduction vs JSON |

## Packages

| Package | Install | Docs | Language |
|---------|---------|------|----------|
| `truth-engine` | `cargo add truth-engine` | [docs.rs](https://docs.rs/truth-engine) | Rust |
| `@temporal-cortex/truth-engine` | `npm i @temporal-cortex/truth-engine` | [README](packages/truth-engine-js/) | JavaScript (WASM) |
| `temporal-cortex-toon` | `cargo add temporal-cortex-toon` | [docs.rs](https://docs.rs/temporal-cortex-toon) | Rust |
| `temporal-cortex-toon-cli` | `cargo install temporal-cortex-toon-cli` | [CLI usage](#toon-cli) | CLI |
| `@temporal-cortex/toon` | `npm i @temporal-cortex/toon` | [README](packages/toon-js/) | JavaScript (WASM) |
| `temporal-cortex-toon` | `pip install temporal-cortex-toon` | [README](crates/toon-python/) | Python |

## Quick Start

### Unified Availability (Rust)

```rust
use truth_engine::{merge_availability, EventStream, ExpandedEvent, PrivacyLevel};
use chrono::{TimeZone, Utc};

// Events from Google Calendar
let google = EventStream {
    stream_id: "google".to_string(),
    events: vec![ExpandedEvent {
        start: Utc.with_ymd_and_hms(2026, 3, 16, 9, 0, 0).unwrap(),
        end: Utc.with_ymd_and_hms(2026, 3, 16, 10, 0, 0).unwrap(),
    }],
};

// Events from Outlook
let outlook = EventStream {
    stream_id: "outlook".to_string(),
    events: vec![ExpandedEvent {
        start: Utc.with_ymd_and_hms(2026, 3, 16, 14, 0, 0).unwrap(),
        end: Utc.with_ymd_and_hms(2026, 3, 16, 15, 0, 0).unwrap(),
    }],
};

// Merge into unified busy/free view
let window_start = Utc.with_ymd_and_hms(2026, 3, 16, 8, 0, 0).unwrap();
let window_end = Utc.with_ymd_and_hms(2026, 3, 16, 17, 0, 0).unwrap();

let availability = merge_availability(
    &[google, outlook],
    window_start,
    window_end,
    PrivacyLevel::Opaque, // Hide which calendar each block came from
);

// availability.busy = [{9:00-10:00}, {14:00-15:00}]
// availability.free = [{8:00-9:00}, {10:00-14:00}, {15:00-17:00}]
```

### RRULE Expansion (Rust)

```rust
use truth_engine::expand_rrule;

// "3rd Tuesday of each month at 2pm Pacific" — deterministic, DST-aware
let events = expand_rrule(
    "FREQ=MONTHLY;BYDAY=TU;BYSETPOS=3",
    "2026-02-17T14:00:00",
    60,                        // 60-minute duration
    "America/Los_Angeles",     // IANA timezone
    Some("2026-12-31T23:59:59"),
    None,
).unwrap();
```

### TOON Compression (Rust)

```rust
use toon_core::{encode, decode};

let json = r#"{"name":"Alice","scores":[95,87,92]}"#;
let toon = encode(json).unwrap();
assert_eq!(toon, "name: Alice\nscores[3]: 95,87,92");

let back = decode(&toon).unwrap();
assert_eq!(back, json); // Perfect roundtrip
```

### JavaScript

```javascript
import { mergeAvailability } from '@temporal-cortex/truth-engine';
import { encode, decode } from '@temporal-cortex/toon';

// Merge availability from multiple calendar sources
const result = mergeAvailability(JSON.stringify([
  { stream_id: "google", events: [{ start: "2026-03-16T09:00:00Z", end: "2026-03-16T10:00:00Z" }] },
  { stream_id: "outlook", events: [{ start: "2026-03-16T14:00:00Z", end: "2026-03-16T15:00:00Z" }] },
]), "2026-03-16T08:00:00Z", "2026-03-16T17:00:00Z", "opaque");

// Compress calendar data for LLM context
const toon = encode('{"summary":"Team Standup","start":"2026-03-16T09:00:00Z"}');
```

### Python

```python
from temporal_cortex_toon import encode, decode, expand_rrule, merge_availability

# Merge availability
result = merge_availability(
    '[{"stream_id":"google","events":[{"start":"2026-03-16T09:00:00Z","end":"2026-03-16T10:00:00Z"}]}]',
    "2026-03-16T08:00:00Z", "2026-03-16T17:00:00Z", "opaque"
)

# Expand recurrence rules
events = expand_rrule("FREQ=WEEKLY;BYDAY=TU,TH", "2026-02-17T14:00:00", 60, "America/Los_Angeles", "2026-06-30T23:59:59", None)

# Compress for LLM consumption
toon = encode('{"name":"Alice","scores":[95,87,92]}')
```

## Truth Engine

The core computation library for calendar operations that LLMs cannot reliably perform:

- **Multi-calendar merging** — merge N event streams into a unified busy/free view with privacy controls (opaque or full)
- **RRULE expansion** — full RFC 5545 support (FREQ, BYDAY, BYSETPOS, COUNT, UNTIL, EXDATE)
- **DST-aware** — events at 14:00 Pacific stay at 14:00 Pacific across DST transitions
- **Conflict detection** — pairwise overlap detection with duration calculation
- **Free/busy computation** — merge busy periods and find available slots
- **Privacy levels** — `Opaque` hides source details (just busy/free); `Full` includes source counts
- **Leap year handling** — `BYMONTHDAY=29` correctly skips non-leap years

## TOON Format

TOON (Token-Oriented Object Notation) is a compact, human-readable format that minimizes token usage when feeding structured data to LLMs.

**JSON** (317 bytes):
```json
{
  "summary": "Team Standup",
  "start": {"dateTime": "2024-01-15T09:00:00-08:00", "timeZone": "America/Los_Angeles"},
  "attendees": [
    {"email": "alice@company.com", "responseStatus": "accepted"},
    {"email": "bob@company.com", "responseStatus": "tentative"}
  ]
}
```

**TOON** (196 bytes, 38% smaller):
```
summary: Team Standup
start:
  dateTime: 2024-01-15T09:00:00-08:00
  timeZone: America/Los_Angeles
attendees[2]{email,responseStatus}:
  alice@company.com,accepted
  bob@company.com,tentative
```

Key features: key folding (indentation replaces braces), tabular arrays (CSV-like rows for uniform objects), inline arrays, and context-dependent quoting.

## TOON CLI

```bash
# Install
cargo install temporal-cortex-toon-cli

# Encode JSON to TOON (stdin to stdout)
echo '{"name":"Alice","age":30}' | toon encode

# Encode from file to file
toon encode -i data.json -o data.toon

# Encode with field filtering (strip noisy fields before encoding)
echo '{"name":"Event","etag":"abc"}' | toon encode --filter etag

# Encode with Google Calendar preset filter
toon encode --filter-preset google -i calendar.json

# Decode TOON back to pretty-printed JSON
toon decode -i data.toon

# Show compression statistics
toon stats -i data.json
```

## Development

This project follows strict **Test-Driven Development** (Red-Green-Refactor). No production code is written without a corresponding test first.

### Prerequisites

- Rust 1.88+ (1.93 recommended) with `wasm32-unknown-unknown` target
- Node.js 18+ with pnpm
- Python 3.12+ (for toon-python bindings)
- `wasm-bindgen-cli` (`cargo install wasm-bindgen-cli`)

### Running Tests

```bash
# All Rust tests
cargo test --workspace

# Individual crates
cargo test -p truth-engine     # availability + expander + conflict + freebusy + proptest
cargo test -p temporal-cortex-toon      # encoder + decoder + roundtrip + spec + proptest
cargo test -p temporal-cortex-toon-cli  # CLI integration tests

# Code quality
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings
cargo deny check

# WASM + JavaScript tests
cargo build -p toon-wasm -p truth-engine-wasm --target wasm32-unknown-unknown --release
wasm-bindgen --target nodejs target/wasm32-unknown-unknown/release/toon_wasm.wasm --out-dir packages/toon-js/wasm/
wasm-bindgen --target nodejs target/wasm32-unknown-unknown/release/truth_engine_wasm.wasm --out-dir packages/truth-engine-js/wasm/
pnpm install && pnpm test

# Python tests
cd crates/toon-python
python3 -m venv .venv && source .venv/bin/activate
pip install maturin pytest
maturin develop
pytest tests/ -v
```

### Repository Structure

```
temporal-cortex-core/
├── crates/
│   ├── truth-engine/        # Availability merging + RRULE + conflict + free/busy
│   ├── truth-engine-wasm/   # WASM bindings for truth-engine
│   ├── toon-core/           # TOON encoder/decoder + semantic filtering
│   ├── toon-cli/            # CLI: toon encode | decode | stats | --filter
│   ├── toon-wasm/           # WASM bindings for toon-core
│   └── toon-python/         # Python bindings via PyO3
├── packages/
│   ├── truth-engine-js/     # @temporal-cortex/truth-engine (NPM)
│   └── toon-js/             # @temporal-cortex/toon (NPM)
└── scripts/poc/             # Test fixtures
```

## Why This Matters for Agent Builders

Every person's availability is fragmented across calendar providers. Google can't see your Outlook calendar. Outlook can't see your iCloud calendar. When AI agents schedule on your behalf, they only see one silo — leading to double-bookings across the others.

Truth Engine provides the deterministic computation layer to merge these fragmented views into a single source of truth. It's the foundation for building scheduling infrastructure that works across calendar boundaries, with privacy controls that let you share availability without exposing event details.

Think of it as DNS for human time — a resolution layer that maps a person's identity to their true availability, regardless of where their calendars live.

## Roadmap

- [ ] Browser/Deno WASM target (currently Node.js only)
- [ ] TOON streaming encoder for large payloads
- [ ] `toon-cli` batch processing for directories
- [ ] Additional calendar filter presets (Outlook, Apple Calendar)
- [ ] Truth Engine: VTODO/VJOURNAL support

See [feature requests](https://github.com/billylui/temporal-cortex-core/issues?q=is%3Aissue+label%3Aenhancement) for community-suggested improvements.

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT OR Apache-2.0
