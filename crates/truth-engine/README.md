# truth-engine

Deterministic calendar computation for AI agents: temporal resolution, timezone conversion, RRULE expansion, availability merging, and conflict detection.

LLMs hallucinate 60% of the time on date/time tasks. They can't reliably answer "What time is next Tuesday at 2pm EST in UTC?" or compute "3rd Tuesday of each month across DST." The Truth Engine replaces inference with deterministic computation — no network, no API keys, just math.

## Usage

```rust
use truth_engine::{expand_rrule, find_conflicts, find_free_slots, ExpandedEvent};

// Expand a recurrence rule into concrete instances
let events = expand_rrule(
    "FREQ=MONTHLY;BYDAY=TU;BYSETPOS=3",       // 3rd Tuesday of each month
    "2026-02-17T14:00:00",                      // start date (local time)
    60,                                          // 60-minute duration
    "America/Los_Angeles",                       // IANA timezone
    Some("2026-06-30T23:59:59"),                 // expand until
    None,                                        // no count limit
).unwrap();

// Each event has start/end as DateTime<Utc>
for event in &events {
    println!("{} → {}", event.start, event.end);
}

// Detect overlapping events between two schedules
let conflicts = find_conflicts(&schedule_a, &schedule_b);
for c in &conflicts {
    println!("Overlap: {} minutes", c.overlap_minutes);
}

// Find free slots in a time window
let free = find_free_slots(
    &busy_events,
    window_start,  // DateTime<Utc>
    window_end,    // DateTime<Utc>
);
```

## Features

### Temporal Computation

- `resolve_relative(anchor, expression, timezone)` — Parse human expressions into RFC 3339 (`"next Tuesday at 2pm"`, `"tomorrow morning"`, `"+2h"`, 60+ patterns)
- `convert_timezone(datetime, timezone)` — DST-aware timezone conversion with offset and DST status
- `compute_duration(start, end)` — Duration breakdown (days, hours, minutes, seconds, human-readable)
- `adjust_timestamp(datetime, adjustment, timezone)` — DST-aware adjustment (compound format: `"+1d2h30m"`)

All functions are pure computation — explicit datetime/anchor parameters, no clock, no state.

### RRULE Expansion

- Full RFC 5545 recurrence rule support via the `rrule` crate v0.14
- `FREQ`: DAILY, WEEKLY, MONTHLY, YEARLY
- `BYDAY`, `BYMONTH`, `BYMONTHDAY`, `BYSETPOS`, `INTERVAL`, `COUNT`, `UNTIL`
- EXDATE exclusions via `expand_rrule_with_exdates()`
- DST-aware: events at 14:00 Pacific stay at 14:00 Pacific across DST transitions (UTC offset shifts automatically)
- Leap year handling: `BYMONTHDAY=29` in February correctly skips non-leap years

### Conflict Detection

- Pairwise overlap detection between two event lists
- Overlap defined as `a.start < b.end && b.start < a.end`
- Adjacent events (end == start) are NOT conflicts
- Returns overlap duration in minutes

### Free/Busy Computation

- Merges overlapping busy periods
- Computes free gaps within a time window
- `find_first_free_slot()` for minimum-duration search

## API

### `resolve_relative(anchor, expression, timezone) -> Result<ResolvedDatetime>`

Resolves human time expressions into precise RFC 3339 timestamps. Supports 60+ patterns across 9 categories (anchored, weekday, time-of-day, explicit time, offsets, combined, period boundaries, ordinals, passthrough).

### `convert_timezone(datetime, target_timezone) -> Result<ConvertedDatetime>`

Converts an RFC 3339 datetime to a target IANA timezone with DST status.

### `compute_duration(start, end) -> Result<DurationInfo>`

Computes the duration between two RFC 3339 timestamps with days/hours/minutes/seconds breakdown.

### `adjust_timestamp(datetime, adjustment, timezone) -> Result<AdjustedTimestamp>`

Adjusts a timestamp by a compound duration (`"+1d2h30m"`), DST-aware for day-level adjustments.

### `expand_rrule(rrule, dtstart, duration_minutes, timezone, until, count)`

Expands an RRULE string into concrete `ExpandedEvent` instances.

### `expand_rrule_with_exdates(rrule, dtstart, duration_minutes, timezone, until, count, exdates)`

Same as above but excludes specific dates (RFC 5545 EXDATE).

### `find_conflicts(events_a, events_b) -> Vec<Conflict>`

Finds all pairwise overlaps between two event lists.

### `find_free_slots(events, window_start, window_end) -> Vec<FreeSlot>`

Computes free time slots within a window, merging overlapping busy periods.

### `find_first_free_slot(events, window_start, window_end, min_duration_minutes) -> Option<FreeSlot>`

Finds the earliest free slot of at least the given duration.

## Architecture

```
temporal.rs     ← Timezone conversion, duration, timestamp adjustment, expression parsing
expander.rs     ← RRULE string → Vec<ExpandedEvent> (wraps rrule + chrono-tz)
availability.rs ← N event streams → unified busy/free with privacy control
conflict.rs     ← Two event lists → Vec<Conflict> (pairwise overlap detection)
freebusy.rs     ← Events + window → Vec<FreeSlot> (gap computation)
dst.rs          ← DstPolicy enum (Skip, ShiftForward, WallClock)
error.rs        ← TruthError enum (InvalidRule, InvalidTimezone, InvalidExpression, etc.)
```

## Testing

150+ tests across six modules:

- **65 temporal tests** — timezone conversion, duration computation, timestamp adjustment, 30+ expression patterns
- **11 expander tests** — CTO's monthly 3rd Tuesday, DST transitions, daily/weekly/biweekly, COUNT, UNTIL
- **25+ availability tests** — multi-stream merging, privacy levels, free slot search
- **7 conflict tests** — overlapping, non-overlapping, adjacent, contained, multiple, empty
- **7 free/busy tests** — single event, merged overlapping, empty, min-duration, fully booked
- **8 RFC 5545 compliance vectors** — biweekly multi-day, yearly, leap year Feb 29, EXDATE, BYSETPOS

```bash
cargo test -p truth-engine
```

## License

MIT OR Apache-2.0
