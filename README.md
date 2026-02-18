# Temporal Cortex Core

[![CI](https://github.com/billylui/temporal-cortex-core/actions/workflows/ci.yml/badge.svg)](https://github.com/billylui/temporal-cortex-core/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/truth-engine.svg)](https://crates.io/crates/truth-engine)
[![npm](https://img.shields.io/npm/v/@temporal-cortex/truth-engine.svg)](https://www.npmjs.com/package/@temporal-cortex/truth-engine)
[![PyPI](https://img.shields.io/pypi/v/temporal-cortex-toon.svg)](https://pypi.org/project/temporal-cortex-toon/)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)

Stop LLMs from hallucinating your calendar. Deterministic RRULE expansion, multi-calendar availability merging, and conflict detection — no inference, no API keys.

## The Problem

LLMs hallucinate **60% of the time** on date, time, and calendar tasks — the worst-performing category in the [AuthenHallu benchmark](https://arxiv.org/abs/2510.10539). Ask a model "When is the 3rd Tuesday of March 2026 at 2pm Pacific in UTC?" and it will confidently give the wrong answer more often than not.

Every person's availability is also fragmented across Google Calendar, Outlook, and iCloud. No single provider sees all of them. AI agents inherit this blindness — leading to double-bookings, missed conflicts, and scheduling drift.

## The Fix

**Truth Engine** is a deterministic computation layer that replaces LLM inference for calendar math: RRULE expansion, DST-aware timezone conversion, multi-calendar availability merging, and conflict detection. No network calls. No API keys. Just math.

**TOON** (Token-Oriented Object Notation) compresses calendar payloads by 40-60% before they enter the context window. Perfect roundtrip fidelity.

## Quick Start

### Python

```bash
pip install temporal-cortex-toon
```

```python
import json
from temporal_cortex_toon import expand_rrule, merge_availability

# Expand a weekly standup: Tuesdays at 2pm Pacific, DST-aware
standup_json = expand_rrule(
    "FREQ=WEEKLY;BYDAY=TU;COUNT=4",
    "2026-03-17T14:00:00",   # local time
    60,                       # 60-minute meetings
    "America/Los_Angeles",    # handles DST transitions
)
standups = json.loads(standup_json)
print(f"{len(standups)} instances expanded")  # 4 Tuesdays, all in UTC

# A one-off dentist appointment from Outlook
outlook_events = [
    {"start": "2026-03-17T22:00:00+00:00", "end": "2026-03-17T23:00:00+00:00"}
]

# Merge both calendars into unified availability
streams = json.dumps([
    {"stream_id": "google",  "events": standups},
    {"stream_id": "outlook", "events": outlook_events},
])
result = json.loads(merge_availability(
    streams,
    "2026-03-17T08:00:00+00:00",  # window start (8am UTC)
    "2026-03-18T00:00:00+00:00",  # window end (midnight UTC)
    True,                          # opaque: hide which calendar each block came from
))
print(f"{len(result['busy'])} busy blocks, {len(result['free'])} free slots")
```

### Rust

```rust
use truth_engine::{expand_rrule, merge_availability, EventStream, ExpandedEvent, PrivacyLevel};
use chrono::{TimeZone, Utc};

// Expand a weekly standup RRULE into concrete UTC instances
let standups = expand_rrule(
    "FREQ=WEEKLY;BYDAY=TU;COUNT=4",
    "2026-03-17T14:00:00",
    60,
    "America/Los_Angeles",
    None,
    None,
).unwrap();

// Merge with a one-off event from another calendar
let availability = merge_availability(
    &[
        EventStream { stream_id: "google".into(), events: standups },
        EventStream {
            stream_id: "outlook".into(),
            events: vec![ExpandedEvent {
                start: Utc.with_ymd_and_hms(2026, 3, 17, 22, 0, 0).unwrap(),
                end: Utc.with_ymd_and_hms(2026, 3, 17, 23, 0, 0).unwrap(),
            }],
        },
    ],
    Utc.with_ymd_and_hms(2026, 3, 17, 8, 0, 0).unwrap(),
    Utc.with_ymd_and_hms(2026, 3, 18, 0, 0, 0).unwrap(),
    PrivacyLevel::Opaque,
);
// availability.busy: merged busy blocks across both calendars
// availability.free: available windows between busy periods
```

## What's Inside

| Feature | Description |
|---------|-------------|
| **RRULE expansion** | RFC 5545 recurrence rules to concrete datetimes. DST-aware, leap-year-safe. |
| **Availability merging** | N event streams from N calendars into one unified busy/free view. |
| **Privacy levels** | `Opaque` (just busy/free) or `Full` (includes source counts per block). |
| **Conflict detection** | Pairwise overlap detection with overlap duration calculation. |
| **Free slot finder** | Find gaps between busy periods, or the first slot of N minutes across all calendars. |
| **TOON encoding** | 40-60% fewer tokens than JSON for calendar payloads. Perfect roundtrip fidelity. |
| **Semantic filtering** | Strip noisy fields (etag, kind, htmlLink) before encoding. Google Calendar preset included. |
| **TOON CLI** | Pipe JSON through `toon encode` / `toon decode` from the command line. |

Pure computation. No network calls. No API keys. No setup. 446+ Rust tests, 39 JS tests, 26 Python tests, ~9,000 property-based tests.

## Installation

**Rust**

```bash
cargo add truth-engine          # calendar computation
cargo add temporal-cortex-toon  # TOON encoder/decoder
```

**JavaScript / TypeScript (Node.js via WASM)**

```bash
npm i @temporal-cortex/truth-engine  # calendar computation
npm i @temporal-cortex/toon          # TOON encoder/decoder
```

**Python (native via PyO3)**

```bash
pip install temporal-cortex-toon  # includes both TOON + Truth Engine functions
```

**CLI**

```bash
cargo install temporal-cortex-toon-cli
```

## Going to Production?

Core handles the math. For production scheduling you also need:

- **Calendar connectors** — Google, Outlook, CalDAV (iCloud) with OAuth token management
- **Booking safety** — Two-Phase Commit with distributed locking to prevent double-bookings
- **Policy engine** — Per-caller access rules, focus time protection, override controls
- **Usage metering** — Track reads, bookings, and expansions per tenant
- **MCP server** — 6 tools ready for AI agent integration

That's the **Temporal Cortex Platform** — currently in private beta. Usage-based pricing starts at $0.001/read and $0.01/booking, with no per-seat fees. Compare that to Nylas ($10/mo + $1/connected account) or Cronofy ($800+/mo).

**[Request early access](https://tally.so/r/aQ66W2)**

## TOON Format

TOON minimizes token usage when feeding structured data to LLMs.

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

Key techniques: indentation replaces braces (key folding), uniform object arrays become CSV-like rows (tabular arrays), and quoting is context-dependent.

```bash
# Encode from stdin
echo '{"name":"Alice","scores":[95,87,92]}' | toon encode

# Filter noisy fields + encode
toon encode --filter-preset google -i calendar.json

# Compression stats
toon stats -i data.json
```

## Architecture

```
temporal-cortex-core/
├── crates/
│   ├── truth-engine/                 # RRULE expansion, availability, conflicts, free/busy
│   ├── truth-engine-wasm/            # WASM bindings
│   ├── temporal-cortex-toon/         # TOON encoder/decoder + semantic filtering
│   ├── temporal-cortex-toon-cli/     # CLI: toon encode | decode | stats
│   ├── temporal-cortex-toon-wasm/    # WASM bindings
│   └── temporal-cortex-toon-python/  # Python bindings via PyO3
├── packages/
│   ├── truth-engine-js/              # @temporal-cortex/truth-engine (npm)
│   └── temporal-cortex-toon-js/      # @temporal-cortex/toon (npm)
└── docs/
```

## Development

### Prerequisites

- Rust 1.88+ with `wasm32-unknown-unknown` target
- Node.js 18+ with pnpm
- Python 3.12+ (for Python bindings)
- `wasm-bindgen-cli` (`cargo install wasm-bindgen-cli`)

### Build & Test

```bash
# Rust (includes ~9,000 property-based tests)
cargo test --workspace
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings
cargo deny check

# WASM + JavaScript
cargo build -p temporal-cortex-toon-wasm -p truth-engine-wasm --target wasm32-unknown-unknown --release
wasm-bindgen --target nodejs target/wasm32-unknown-unknown/release/toon_wasm.wasm --out-dir packages/temporal-cortex-toon-js/wasm/
wasm-bindgen --target nodejs target/wasm32-unknown-unknown/release/truth_engine_wasm.wasm --out-dir packages/truth-engine-js/wasm/
pnpm install && pnpm test

# Python
cd crates/temporal-cortex-toon-python
python3 -m venv .venv && source .venv/bin/activate
pip install maturin pytest
maturin develop
pytest tests/ -v
```

This project follows strict TDD (Red-Green-Refactor). No production code without a corresponding test.

## Contributing

Contributions welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for setup, testing, and PR guidelines.

## License

Licensed under either of

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)

at your option.
