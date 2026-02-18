# temporal-cortex-toon (Python)

Python bindings for the TOON format encoder/decoder and truth-engine, built with PyO3 and maturin.

## Installation

```bash
pip install temporal-cortex-toon
```

## Usage

```python
from temporal_cortex_toon import encode, decode, filter_and_encode, expand_rrule

# JSON → TOON
toon = encode('{"name":"Alice","scores":[95,87,92]}')
print(toon)
# name: Alice
# scores[3]: 95,87,92

# TOON → JSON (perfect roundtrip)
json_str = decode(toon)
print(json_str)
# {"name":"Alice","scores":[95,87,92]}

# Semantic filtering: strip noisy fields before encoding
toon = filter_and_encode(
    '{"name":"Event","etag":"abc","kind":"calendar#event"}',
    ["etag", "kind"],
)
print(toon)
# name: Event

# RRULE expansion
import json
events_json = expand_rrule(
    "FREQ=WEEKLY;BYDAY=TU,TH",       # RFC 5545 RRULE
    "2026-02-17T14:00:00",            # start date (local time)
    60,                                # duration in minutes
    "America/Los_Angeles",             # IANA timezone
    "2026-06-30T23:59:59",            # expand until (optional)
    None,                              # max count (optional)
)
events = json.loads(events_json)
for e in events:
    print(f"{e['start']} → {e['end']}")
```

## API

### `encode(json: str) -> str`

Converts a valid JSON string into TOON format. Raises `ValueError` if the input is not valid JSON.

### `decode(toon: str) -> str`

Converts a TOON string back into compact JSON. Raises `ValueError` if the input is not valid TOON.

### `filter_and_encode(json: str, patterns: list[str]) -> str`

Strips fields matching the given patterns from JSON, then encodes to TOON. Patterns support:
- `"etag"` — strip the top-level field
- `"items.etag"` — strip nested field via dot-path
- `"*.etag"` — wildcard: strip field at any depth

### `expand_rrule(rrule, dtstart, duration_minutes, timezone, until=None, max_count=None) -> str`

Expands an RFC 5545 RRULE into concrete event instances. Returns a JSON string containing an array of `{"start": "...", "end": "..."}` objects with UTC datetimes.

## Build from Source

```bash
# From the crate directory:
cd crates/temporal-cortex-toon-python

# Create a virtualenv and install
python3 -m venv .venv
source .venv/bin/activate
pip install maturin pytest

# Build and install the native extension
maturin develop

# Run tests
pytest tests/ -v
```

## Testing

26 pytest tests across 5 suites:

- **9 encode tests** — simple objects, nested, arrays, empty, null, booleans, strings
- **3 decode tests** — simple, nested, valid JSON output
- **3 roundtrip tests** — simple, nested, type preservation
- **4 filter tests** — field removal, empty patterns, wildcards, error handling
- **7 RRULE tests** — daily count, start/end fields, until, max count, weekly, error handling

```bash
cd crates/temporal-cortex-toon-python
source .venv/bin/activate
pytest tests/ -v
```

## Architecture

```
src/lib.rs       ← PyO3 #[pyfunction] wrappers around temporal-cortex-toon and truth-engine
pyproject.toml   ← maturin build configuration
tests/           ← pytest test suite
```

The Python module (`temporal_cortex_toon`) is a thin wrapper that:
1. Accepts Python strings
2. Calls the underlying Rust functions (temporal-cortex-toon encode/decode, truth-engine expand)
3. Maps Rust errors to Python `ValueError` exceptions

## License

MIT OR Apache-2.0
