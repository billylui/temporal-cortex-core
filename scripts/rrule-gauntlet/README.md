# The RRULE Gauntlet

**10 calendar challenges that break LLMs on date/time computation.**

LLMs hallucinate dates. The [AuthenHallu benchmark](https://arxiv.org/abs/2510.10539) found a 60% hallucination rate on temporal reasoning tasks, but no public benchmark exists specifically for RFC 5545 RRULE expansion -- the core operation behind every recurring calendar event. The RRULE Gauntlet fills that gap: 10 carefully designed challenges that target specific, reproducible LLM failure modes, each with an objectively verifiable correct answer computed by [Truth Engine](https://github.com/billylui/temporal-cortex-core).

## The Challenges

| # | Challenge | Difficulty | Trap |
|---|-----------|-----------|------|
| 1 | DST Spring-Forward Gap | Hard | 2:00 AM doesn't exist on spring-forward day; UTC offset shifts |
| 2 | DST Fall-Back Ambiguity | Hard | 1:30 AM occurs twice on fall-back day |
| 3 | Last Weekday of Month | Hard | BYSETPOS=-1 requires knowing the last calendar day AND its weekday |
| 4 | EXDATE Exclusions | Medium | Enumerate Tuesdays in range, then subtract 3 cancelled dates |
| 5 | Tri-Weekly COUNT | Medium | 21-day gaps crossing month boundaries + CST/CDT shift |
| 6 | Cross-Year Boundary | Easy | Year transition Dec 2025 to Jan 2026 |
| 7 | First Monday of Quarter | Hard | Calendar lookup for 4 months + EST/EDT offset variation |
| 8 | UNTIL Timezone Boundary | Hard | UNTIL in UTC excludes a local-date occurrence due to offset |
| 9 | Every Weekday (15 days) | Easy | DST shift mid-sequence changes UTC offset for 10 of 15 answers |
| 10 | Monthly on the 29th | Medium | February 2025 skipped (not a leap year) |

## Quick Start

### Prerequisites

```bash
pip install temporal-cortex-toon
```

### Verify the Dataset

Confirm all 10 challenges produce the correct answers via Truth Engine:

```bash
python rrule_gauntlet.py verify --verbose
```

### Test an LLM

```bash
# OpenAI
OPENAI_API_KEY=sk-... python rrule_gauntlet.py test --model gpt-4o --output results.json

# Anthropic
ANTHROPIC_API_KEY=sk-... python rrule_gauntlet.py test --provider anthropic --model claude-sonnet-4-20250514 --output results.json
```

### View Prompts

See the exact prompt sent to LLMs for any challenge:

```bash
python rrule_gauntlet.py prompt --challenge dst-spring-forward
```

## How It Works

1. **challenges.json** contains 10 RRULE expansion challenges with correct answers pre-computed by Truth Engine (a Rust RFC 5545 implementation).
2. **rrule_gauntlet.py** can verify the dataset (`verify`), display prompts (`prompt`), or test LLMs (`test`).
3. LLMs receive the RRULE string, DTSTART, timezone, and a natural language question. They must return UTC datetime strings.
4. Answers are compared element-by-element against the Truth Engine output.

## Challenge Deep Dives

### Challenge 1: DST Spring-Forward Gap

A weekly Sunday event at 2:00 AM Eastern Time. On March 8, 2026, clocks spring forward at 2:00 AM -- that time doesn't exist. The engine resolves the gap at 07:00 UTC for March 8, then all subsequent occurrences use EDT (06:00 UTC). LLMs typically either keep all occurrences at 07:00 UTC (ignoring DST) or shift too early.

### Challenge 2: DST Fall-Back Ambiguity

A daily event at 1:30 AM Eastern Time. On November 1, 2026, 1:30 AM occurs twice (EDT then EST). The engine picks the first (EDT) occurrence, keeping 05:30 UTC. Only November 2 shifts to 06:30 UTC. LLMs rarely even recognize the ambiguity.

### Challenge 3: Last Weekday of Month

`BYSETPOS=-1` with `BYDAY=MO,TU,WE,TH,FR` means "the last weekday." This is NOT "the last Friday." March 31, 2026 is a Tuesday. April 30 is a Thursday. LLMs that default to Friday get 4 of 6 wrong.

### Challenge 4: EXDATE Exclusions

A weekly Tuesday meeting with 3 cancelled dates. There are 6 Tuesdays in the range (Mar 3, 10, 17, 24, 31, Apr 7). Subtract the 3 EXDATEs (Mar 10, 17, 31) to get 3 remaining. LLMs often miscount the total Tuesdays in range or apply the exclusions to the wrong dates.

### Challenge 5: Tri-Weekly COUNT

Every 3rd week means 21-day gaps, not every 3rd day. Starting Jan 5 in Central Time, the sequence crosses month boundaries (Jan 5 -> Jan 26 -> Feb 16 -> Mar 9 -> Mar 30) and the DST spring-forward on March 8 shifts the UTC offset from CST (UTC-6) to CDT (UTC-5) mid-sequence.

### Challenge 6: Cross-Year Boundary

Weekly Sundays starting December 28, 2025. The year boundary (2025 -> 2026) is conceptually simple, but LLMs sometimes skip or duplicate dates near January 1. Europe/London is GMT in winter (UTC+0), so the timezone is trivial -- the trap is purely the year rollover arithmetic.

### Challenge 7: First Monday of Quarter

`BYDAY=1MO` with `FREQ=YEARLY` and `BYMONTH=1,4,7,10` requires a 2026 calendar lookup: Jan 5, Apr 6, Jul 6, Oct 5. LLMs must also handle EST (UTC-5) for January and October vs EDT (UTC-4) for April and July. Getting the dates right but the offsets wrong means 2 of 4 answers are off by an hour.

### Challenge 8: UNTIL Timezone Boundary

DTSTART is 10 PM Pacific (= 6 AM next day UTC). UNTIL is Jan 15 at 04:59:59 UTC. The Jan 14 local occurrence maps to Jan 15 06:00 UTC, which exceeds UNTIL. Result: 4 occurrences, not 5. The cross-day UTC mapping is the trap.

### Challenge 9: Every Weekday

15 weekdays starting March 2 at 9 AM Eastern Time seems trivial. But DST spring-forward on March 8 means the first 5 days are at 14:00 UTC (EST) and the last 10 are at 13:00 UTC (EDT). LLMs that ignore the mid-sequence offset change get 10 of 15 wrong.

### Challenge 10: Monthly on the 29th

`BYMONTHDAY=29` starting January 2025. February 2025 has only 28 days (not a leap year), so it is skipped entirely per RFC 5545. The 8 occurrences are: Jan, Mar, Apr, May, Jun, Jul, Aug, Sep. LLMs either fabricate a February 29, substitute February 28, or miscount the total after the skip.

## License

MIT OR Apache-2.0 (same as temporal-cortex-core)
