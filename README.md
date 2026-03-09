# Temporal Cortex Core

[![CI](https://github.com/temporal-cortex/core/actions/workflows/ci.yml/badge.svg)](https://github.com/temporal-cortex/core/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/truth-engine.svg)](https://crates.io/crates/truth-engine)
[![npm](https://img.shields.io/npm/v/@temporal-cortex/truth-engine.svg)](https://www.npmjs.com/package/@temporal-cortex/truth-engine)
[![PyPI](https://img.shields.io/pypi/v/temporal-cortex-toon.svg)](https://pypi.org/project/temporal-cortex-toon/)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)

**v0.3.1** · [Changelog](CHANGELOG.md) · **Website:** [temporal-cortex.com](https://temporal-cortex.com)

Core is the computation engine behind [Temporal Cortex](https://temporal-cortex.com) — the scheduling layer that lets AI agents schedule with anyone.

Temporal Cortex Core is a deterministic computation library that replaces LLM inference for calendar math. It provides temporal resolution ("next Tuesday at 2pm" → RFC 3339), RFC 5545 RRULE expansion, multi-calendar availability merging, conflict detection, and TOON token compression — available for Rust, JavaScript/WASM, and Python. No network calls, no API keys. Used by the [Temporal Cortex MCP server](https://github.com/temporal-cortex/mcp).

## Why do LLMs fail at calendar computation?

Even the latest LLMs — GPT-5, Claude, Gemini — **score below 50%** on temporal reasoning tasks ([OOLONG benchmark](https://arxiv.org/abs/2511.02817)). Earlier models scored as low as 29% on scheduling and 13% on duration calculations ([Test of Time, ICLR 2025](https://arxiv.org/abs/2406.09170)). Ask a model "When is the 3rd Tuesday of March 2026 at 2pm Pacific in UTC?" and it will confidently give the wrong answer more often than not.

Every person's availability is also fragmented across Google Calendar, Outlook, and iCloud. No single provider sees all of them. AI agents inherit this blindness — leading to double-bookings, missed conflicts, and scheduling drift.

## How does Temporal Cortex Core solve this?

**Truth Engine** is a deterministic computation layer that replaces LLM inference for calendar math: temporal resolution (`"next Tuesday at 2pm"` → RFC 3339), timezone conversion, duration computation, RRULE expansion, multi-calendar availability merging, and conflict detection. No network calls. No API keys. Just math.

**TOON** (Token-Oriented Object Notation) compresses calendar payloads by ~40% before they enter the context window. Perfect roundtrip fidelity.

For a ready-to-use Model Context Protocol server with these capabilities built in, see [Temporal Cortex MCP](https://github.com/temporal-cortex/mcp).

## How do I use Temporal Cortex Core?

**Use Temporal Cortex Core in 3 steps:**

1. **Install** for your language — Rust, JavaScript, or Python (see installation section below).
2. **Expand events** — use `expand_rrule()` to turn recurrence rules into concrete datetime instances with correct DST handling.
3. **Merge availability** — use `merge_availability()` to combine event streams from multiple calendars into a unified busy/free view.

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

## What features does Temporal Cortex Core include?

| Feature | Description |
|---------|-------------|
| **Temporal context** | Timezone conversion, duration computation, timestamp adjustment, relative datetime resolution (`"next Tuesday at 2pm"` → RFC 3339). |
| **RRULE expansion** | RFC 5545 recurrence rules to concrete datetimes. DST-aware, leap-year-safe. |
| **Availability merging** | N event streams from N calendars into one unified busy/free view. |
| **Privacy levels** | `Opaque` (just busy/free) or `Full` (includes source counts per block). |
| **Conflict detection** | Pairwise overlap detection with overlap duration calculation. |
| **Free slot finder** | Find gaps between busy periods, or the first slot of N minutes across all calendars. |
| **TOON encoding** | ~40% fewer tokens than JSON for calendar payloads. Perfect roundtrip fidelity. |
| **Semantic filtering** | Strip noisy fields (etag, kind, htmlLink) before encoding. Google Calendar preset included. |
| **TOON CLI** | Pipe JSON through `toon encode` / `toon decode` from the command line. |

Pure computation. No network calls. No API keys. No setup. 510+ Rust tests, 42 JS tests, 30 Python tests, ~9,000 property-based tests.

## How do I install Temporal Cortex Core?

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

## How do I use Temporal Cortex Core in production?

`merge_availability()` and all Core functions work in any production environment. For production calendar integrations, three additional challenges arise:

- **OAuth token refresh** across Google Calendar, Microsoft Outlook, and CalDAV — each with different scopes, error codes, and rate limits
- **Provider differences** — Google returns RFC 3339, Outlook returns truncated UTC, CalDAV uses its own format conventions
- **Race conditions** — two agents booking the same slot simultaneously without distributed locking

The [Temporal Cortex MCP server](https://github.com/temporal-cortex/mcp) handles all of this: managed OAuth connectors, Two-Phase Commit with distributed locking, and multi-calendar availability merging. A managed platform is available at [app.temporal-cortex.com](https://app.temporal-cortex.com).

## What is TOON and how does it reduce token usage?

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

## How does the temporal computation module work?

The `temporal` module provides four pure functions for datetime work that LLMs get wrong:

### How do I resolve human datetime expressions?

```rust
use truth_engine::temporal::{resolve_relative, ResolvedDatetime};
use chrono::Utc;

let now = Utc::now();
let result = resolve_relative(now, "next Tuesday at 2pm", "America/New_York").unwrap();
println!("{}", result.resolved_local);  // "2026-02-24T14:00:00-05:00"
println!("{}", result.interpretation);  // "Tuesday, February 24, 2026 at 2:00 PM"
```

Supports: `"tomorrow morning"`, `"in 2 hours"`, `"last Friday"`, `"end of month"`, `"start of last week"`, `"end of next quarter"`, `"third Tuesday of March"`, `"+1d2h30m"`, and [70+ expression patterns](crates/truth-engine/src/temporal.rs).

Configurable week start (Monday default, Sunday option):

```rust
use truth_engine::temporal::{resolve_relative_with_options, ResolveOptions, WeekStartDay};
use chrono::Utc;

let now = Utc::now();
let options = ResolveOptions { week_start: WeekStartDay::Sunday };
let result = resolve_relative_with_options(now, "start of week", "America/New_York", &options).unwrap();
// Returns Sunday 00:00 instead of Monday 00:00
```

### How do I convert between timezones?

```rust
use truth_engine::temporal::convert_timezone;

let result = convert_timezone("2026-03-08T06:00:00+00:00", "America/New_York").unwrap();
assert_eq!(result.local, "2026-03-08T01:00:00-05:00");
assert_eq!(result.dst_active, false);
```

### How do I compute duration between timestamps?

```rust
use truth_engine::temporal::compute_duration;

let d = compute_duration(
    "2026-02-20T09:00:00+00:00",
    "2026-02-20T17:30:00+00:00",
).unwrap();
assert_eq!(d.hours, 8);
assert_eq!(d.minutes, 30);
assert_eq!(d.human_readable, "8 hours, 30 minutes");
```

### How do I adjust timestamps across DST?

```rust
use truth_engine::temporal::adjust_timestamp;

let result = adjust_timestamp(
    "2026-03-08T01:00:00-05:00",  // 1am EST
    "+1d",                         // add one day (across DST spring-forward)
    "America/New_York",
).unwrap();
// Same wall-clock time, different offset (DST-aware)
assert!(result.adjusted_local.contains("01:00:00-04:00"));
```

All four functions are pure computation — no clock, no network. They take explicit datetime/anchor parameters and return deterministic results. Available in Rust, WASM/JavaScript, and Python.

## What is the crate architecture?

```
core/
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

## Frequently Asked Questions

### Does Temporal Cortex Core require network access or API keys?

No. All computation is pure and deterministic. The library takes explicit datetime parameters and returns results using only CPU computation. There are no network calls, no API keys, and no external dependencies beyond the Rust standard library and chrono/chrono-tz.

### What RRULE edge cases does Truth Engine handle?

Truth Engine handles DST transitions (events at 2pm Pacific stay at 2pm Pacific year-round), BYSETPOS=-1 (last occurrence of a weekday per month), EXDATE with timezone offsets, INTERVAL>1 with multi-day BYDAY, and February 29 yearly recurrences (correctly skipping non-leap years). All behaviors follow RFC 5545 strictly.

### How does availability merging work across calendars?

Pass N event streams from N calendars to `merge_availability()`. The function merges overlapping busy periods, computes free gaps within a time window, and returns a unified busy/free view. Privacy levels control whether the output reveals which calendar each busy block came from (Full) or only shows aggregated busy/free status (Opaque).

### What languages and platforms are supported?

Rust (`cargo add truth-engine`), JavaScript/TypeScript (`npm i @temporal-cortex/truth-engine`, WASM-based), and Python (`pip install temporal-cortex-toon`, PyO3 native bindings). The TOON encoder/decoder is available as a separate package in all three ecosystems. A CLI (`toon encode` / `toon decode` / `toon stats`) is also available via `cargo install temporal-cortex-toon-cli`.

### What is the difference between Core and the MCP server?

Core is the computation library — it provides the math for temporal resolution, RRULE expansion, availability merging, and TOON encoding. The [MCP server](https://github.com/temporal-cortex/mcp) wraps Core in a Model Context Protocol interface, adds calendar provider connectors (Google Calendar, Microsoft Outlook, CalDAV), and provides Two-Phase Commit booking. Use Core directly when you need the computation without MCP infrastructure.

### How does TOON compare to JSON for LLM context windows?

TOON compresses structured data by ~40% compared to JSON while maintaining perfect roundtrip fidelity (encode then decode produces identical output). Key techniques include indentation-based nesting (replacing braces), tabular arrays for uniform objects (replacing repeated keys), and context-dependent quoting. A Google Calendar event payload typically compresses by 38%.

## How do I build and test Temporal Cortex Core?

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

## Where can I learn more about Temporal Cortex?

- **[temporal-cortex/mcp](https://github.com/temporal-cortex/mcp)** — MCP server (18 tools, 5 layers) powered by Truth Engine and TOON
- **[temporal-cortex/skills](https://github.com/temporal-cortex/skills)** — Agent Skills that teach AI agents the scheduling workflow

## Contributing

Contributions welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for setup, testing, and PR guidelines.

## License

Licensed under either of

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)

at your option.
