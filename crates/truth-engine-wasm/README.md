# truth-engine-wasm

WASM bindings for the truth-engine, exposing RRULE expansion, conflict detection, and free/busy computation to JavaScript/TypeScript via `wasm-bindgen`.

## Usage

```typescript
import { expandRRule, findConflicts, findFreeSlots } from "./truth_engine_wasm";

// Expand a recurrence rule
const eventsJson = expandRRule(
  "FREQ=WEEKLY;BYDAY=TU,TH",
  "2026-02-17T14:00:00",
  60,                          // duration in minutes
  "America/Los_Angeles",
  "2026-06-30T23:59:59",       // until (optional)
  undefined,                    // max_count (optional)
);
const events = JSON.parse(eventsJson);
// [{ start: "2026-02-17T22:00:00+00:00", end: "2026-02-17T23:00:00+00:00" }, ...]

// Find conflicts between two schedules
const conflictsJson = findConflicts(
  JSON.stringify(scheduleA),
  JSON.stringify(scheduleB),
);
const conflicts = JSON.parse(conflictsJson);
// [{ event_a: {...}, event_b: {...}, overlap_minutes: 30 }, ...]

// Find free slots in a time window
const freeSlotsJson = findFreeSlots(
  JSON.stringify(busyEvents),
  "2026-02-17T09:00:00",
  "2026-02-17T17:00:00",
);
const slots = JSON.parse(freeSlotsJson);
// [{ start: "...", end: "...", duration_minutes: 60 }, ...]
```

## API

All functions accept and return JSON strings for complex types, matching the pattern used by `temporal-cortex-toon-wasm`.

### `expandRRule(rrule, dtstart, durationMinutes, timezone, until?, maxCount?)`

Expands an RRULE into concrete event instances. Returns a JSON array of `{start, end}` objects with RFC 3339 datetime strings.

### `findConflicts(eventsAJson, eventsBJson)`

Finds overlapping events between two schedules. Both inputs are JSON arrays of `{start, end}` objects. Returns a JSON array of conflict objects.

### `findFreeSlots(eventsJson, windowStart, windowEnd)`

Computes free time slots within a window. Returns a JSON array of `{start, end, duration_minutes}` objects.

## Build from Source

```bash
# From the monorepo root:

# 1. Build the WASM binary
cargo build -p truth-engine-wasm --target wasm32-unknown-unknown --release

# 2. Generate Node.js bindings
wasm-bindgen --target nodejs \
  --out-dir packages/truth-engine-js/wasm/ \
  target/wasm32-unknown-unknown/release/truth_engine_wasm.wasm

# 3. Rename for ESM/CJS compatibility
mv packages/truth-engine-js/wasm/truth_engine_wasm.js \
   packages/truth-engine-js/wasm/truth_engine_wasm.cjs
```

## Architecture

```
src/lib.rs  ← #[wasm_bindgen] exports wrapping truth-engine Rust API
            ← DTO layer for JSON serialization across WASM boundary
            ← ISO 8601 datetime parsing (RFC 3339 + naive formats)
```

The WASM functions are thin wrappers that:
1. Deserialize JSON string arguments into Rust types
2. Call the underlying truth-engine functions
3. Serialize results back to JSON strings

## License

MIT OR Apache-2.0
