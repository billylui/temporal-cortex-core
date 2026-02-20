import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  expandRRule, findConflicts, findFreeSlots, mergeAvailability, _resetHint,
  convertTimezone, computeDuration, adjustTimestamp, resolveRelative,
} from "../src/index.js";

describe("expandRRule", () => {
  it("expands a daily rule with COUNT", () => {
    const events = expandRRule("FREQ=DAILY;COUNT=3", "2026-02-17T14:00:00", 60, "UTC");
    expect(events).toHaveLength(3);
    expect(events[0].start).toContain("2026-02-17");
    expect(events[1].start).toContain("2026-02-18");
    expect(events[2].start).toContain("2026-02-19");
  });

  it("respects timezone (PST vs PDT)", () => {
    // Feb is PST (UTC-8), so 14:00 local = 22:00 UTC
    const events = expandRRule("FREQ=DAILY;COUNT=1", "2026-02-17T14:00:00", 60, "America/Los_Angeles");
    expect(events[0].start).toContain("22:00:00");
  });

  it("applies duration correctly", () => {
    const events = expandRRule("FREQ=DAILY;COUNT=1", "2026-02-17T14:00:00", 90, "UTC");
    expect(events[0].start).toContain("14:00:00");
    expect(events[0].end).toContain("15:30:00");
  });

  it("expands weekly with BYDAY", () => {
    // 2026-02-16 is a Monday
    const events = expandRRule("FREQ=WEEKLY;COUNT=4;BYDAY=MO", "2026-02-16T09:00:00", 60, "UTC");
    expect(events).toHaveLength(4);
  });

  it("respects UNTIL boundary", () => {
    // DAILY from Feb 17, UNTIL Feb 20 23:59:59 -> should get Feb 17, 18, 19, 20
    const events = expandRRule("FREQ=DAILY", "2026-02-17T14:00:00", 60, "UTC", "2026-02-20T23:59:59", undefined);
    expect(events).toHaveLength(4);
    expect(events[0].start).toContain("2026-02-17");
    expect(events[3].start).toContain("2026-02-20");
  });

  it("respects maxCount", () => {
    const events = expandRRule("FREQ=DAILY", "2026-02-17T14:00:00", 60, "UTC", undefined, 5);
    expect(events).toHaveLength(5);
  });

  it("throws on invalid RRULE", () => {
    expect(() => expandRRule("", "2026-02-17T14:00:00", 60, "UTC")).toThrow();
  });

  it("throws on invalid timezone", () => {
    expect(() => expandRRule("FREQ=DAILY;COUNT=1", "2026-02-17T14:00:00", 60, "Not/Real")).toThrow();
  });
});

describe("findConflicts", () => {
  it("detects overlapping events", () => {
    const a = [{ start: "2026-02-17T14:00:00+00:00", end: "2026-02-17T15:00:00+00:00" }];
    const b = [{ start: "2026-02-17T14:30:00+00:00", end: "2026-02-17T15:30:00+00:00" }];
    const conflicts = findConflicts(a, b);
    expect(conflicts).toHaveLength(1);
    expect(conflicts[0].overlap_minutes).toBe(30);
  });

  it("returns empty for non-overlapping", () => {
    const a = [{ start: "2026-02-17T14:00:00+00:00", end: "2026-02-17T15:00:00+00:00" }];
    const b = [{ start: "2026-02-17T16:00:00+00:00", end: "2026-02-17T17:00:00+00:00" }];
    expect(findConflicts(a, b)).toHaveLength(0);
  });

  it("adjacent events are NOT conflicts", () => {
    const a = [{ start: "2026-02-17T14:00:00+00:00", end: "2026-02-17T15:00:00+00:00" }];
    const b = [{ start: "2026-02-17T15:00:00+00:00", end: "2026-02-17T16:00:00+00:00" }];
    expect(findConflicts(a, b)).toHaveLength(0);
  });
});

describe("findFreeSlots", () => {
  it("finds gaps between events", () => {
    const events = [
      { start: "2026-02-17T09:00:00+00:00", end: "2026-02-17T10:00:00+00:00" },
      { start: "2026-02-17T11:00:00+00:00", end: "2026-02-17T12:00:00+00:00" },
    ];
    const slots = findFreeSlots(events, "2026-02-17T08:00:00", "2026-02-17T13:00:00");
    // Free slots: [08:00-09:00], [10:00-11:00], [12:00-13:00]
    expect(slots).toHaveLength(3);
    expect(slots[0].duration_minutes).toBe(60);
    expect(slots[1].duration_minutes).toBe(60);
    expect(slots[2].duration_minutes).toBe(60);
  });

  it("returns full window when no events", () => {
    const slots = findFreeSlots([], "2026-02-17T09:00:00", "2026-02-17T17:00:00");
    expect(slots).toHaveLength(1);
    expect(slots[0].duration_minutes).toBe(480); // 8 hours
  });
});

describe("mergeAvailability hint", () => {
  beforeEach(() => {
    _resetHint();
    vi.restoreAllMocks();
    delete process.env.TEMPORAL_CORTEX_QUIET;
  });

  it("emits console.info with 3+ streams", () => {
    const spy = vi.spyOn(console, "info").mockImplementation(() => {});
    const streams = [
      { stream_id: "a", events: [] },
      { stream_id: "b", events: [] },
      { stream_id: "c", events: [] },
    ];
    mergeAvailability(streams, "2026-03-17T08:00:00+00:00", "2026-03-18T00:00:00+00:00");
    expect(spy).toHaveBeenCalledOnce();
    expect(spy.mock.calls[0][0]).toContain("tally.so/r/aQ66W2");
  });

  it("does not emit with 2 streams", () => {
    const spy = vi.spyOn(console, "info").mockImplementation(() => {});
    const streams = [
      { stream_id: "a", events: [] },
      { stream_id: "b", events: [] },
    ];
    mergeAvailability(streams, "2026-03-17T08:00:00+00:00", "2026-03-18T00:00:00+00:00");
    expect(spy).not.toHaveBeenCalled();
  });

  it("fires only once per session", () => {
    const spy = vi.spyOn(console, "info").mockImplementation(() => {});
    const streams = [
      { stream_id: "a", events: [] },
      { stream_id: "b", events: [] },
      { stream_id: "c", events: [] },
    ];
    mergeAvailability(streams, "2026-03-17T08:00:00+00:00", "2026-03-18T00:00:00+00:00");
    mergeAvailability(streams, "2026-03-17T08:00:00+00:00", "2026-03-18T00:00:00+00:00");
    expect(spy).toHaveBeenCalledOnce();
  });
});

// ---------------------------------------------------------------------------
// Temporal computation
// ---------------------------------------------------------------------------

describe("convertTimezone", () => {
  it("converts UTC to Eastern", () => {
    const result = convertTimezone("2026-03-15T14:00:00Z", "America/New_York");
    expect(result.timezone).toBe("America/New_York");
    expect(result.local).toContain("10:00:00");
    expect(result.dst_active).toBe(true); // March = EDT
  });

  it("throws on invalid timezone", () => {
    expect(() => convertTimezone("2026-03-15T14:00:00Z", "Invalid/Zone")).toThrow();
  });
});

describe("computeDuration", () => {
  it("computes 8-hour duration", () => {
    const result = computeDuration("2026-03-16T09:00:00Z", "2026-03-16T17:00:00Z");
    expect(result.total_seconds).toBe(28800);
    expect(result.hours).toBe(8);
    expect(result.days).toBe(0);
  });

  it("throws on invalid input", () => {
    expect(() => computeDuration("not-a-date", "2026-03-16T17:00:00Z")).toThrow();
  });
});

describe("adjustTimestamp", () => {
  it("adds 2 hours", () => {
    const result = adjustTimestamp("2026-03-16T10:00:00Z", "+2h", "UTC");
    expect(result.adjusted_utc).toContain("12:00:00");
    expect(result.adjustment_applied).toBe("+2h");
  });

  it("throws on invalid format", () => {
    expect(() => adjustTimestamp("2026-03-16T10:00:00Z", "2h", "UTC")).toThrow();
  });
});

describe("resolveRelative", () => {
  const anchor = "2026-02-18T14:30:00+00:00"; // Wednesday

  it("resolves 'tomorrow' to next day", () => {
    const result = resolveRelative(anchor, "tomorrow", "UTC");
    expect(result.resolved_utc).toContain("2026-02-19");
    expect(result.resolved_utc).toContain("00:00:00");
  });

  it("resolves 'next Tuesday at 2pm'", () => {
    const result = resolveRelative(anchor, "next Tuesday at 2pm", "UTC");
    expect(result.resolved_utc).toContain("2026-02-24");
    expect(result.resolved_utc).toContain("14:00:00");
  });

  it("throws on unparseable expression", () => {
    expect(() => resolveRelative(anchor, "gobbledygook", "UTC")).toThrow();
  });
});
