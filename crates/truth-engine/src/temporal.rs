//! Deterministic temporal computation for AI agents.
//!
//! Provides pure functions for timezone conversion, duration computation,
//! timestamp adjustment, and relative datetime resolution. All functions
//! take explicit inputs (no system clock access) — the caller provides
//! the "now" anchor when needed, keeping these functions testable and
//! WASM-compatible.
//!
//! # Design Principle
//!
//! These functions replace LLM inference with deterministic computation.
//! If an expression cannot be parsed unambiguously, we return an error
//! rather than guessing — the opposite of what LLMs do.
//!
//! # Functions
//!
//! - [`convert_timezone`] — Convert a datetime between timezone representations
//! - [`compute_duration`] — Calculate the duration between two timestamps
//! - [`adjust_timestamp`] — Add or subtract a duration from a timestamp
//! - [`resolve_relative`] — Resolve a relative time expression to an absolute datetime
//!
//! # Datetime Accuracy
//!
//! When used via the MCP server, the "now" anchor comes from `chrono::Utc::now()`,
//! which reads the OS kernel clock (NTP-synchronized on modern systems, typically
//! <50ms accuracy). No online time service is used.

use chrono::{DateTime, Datelike, NaiveDate, NaiveTime, Offset, TimeZone, Utc, Weekday};
use chrono_tz::Tz;
use serde::Serialize;

use crate::error::TruthError;

// ── Configurable week start ─────────────────────────────────────────────────

/// Which day begins a week for period computations ("start of week", "next week", etc.).
///
/// Does **not** affect named-weekday expressions like "next Monday" or "last Friday".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub enum WeekStartDay {
    /// ISO 8601 standard (Monday = day 0 of the week).
    #[default]
    Monday,
    /// US/Canada convention (Sunday = day 0 of the week).
    Sunday,
}

/// Options for [`resolve_relative_with_options`].
#[derive(Debug, Clone, Default)]
pub struct ResolveOptions {
    /// Which day starts the week for period computations.
    pub week_start: WeekStartDay,
}

/// How many days `weekday` is from the week-start day.
fn days_from_week_start(weekday: Weekday, week_start: WeekStartDay) -> i64 {
    match week_start {
        WeekStartDay::Monday => weekday.num_days_from_monday() as i64,
        WeekStartDay::Sunday => weekday.num_days_from_sunday() as i64,
    }
}

// ── convert_timezone ────────────────────────────────────────────────────────

/// The result of converting a datetime to a target timezone.
#[derive(Debug, Clone, Serialize)]
pub struct ConvertedDatetime {
    /// The instant in UTC (RFC 3339).
    pub utc: String,
    /// The instant in the target timezone (RFC 3339 with offset).
    pub local: String,
    /// The IANA timezone name used.
    pub timezone: String,
    /// The UTC offset at this instant (e.g., "-05:00").
    pub utc_offset: String,
    /// Whether Daylight Saving Time is active at this instant.
    pub dst_active: bool,
}

/// Convert a datetime string to a different timezone representation.
///
/// # Arguments
///
/// * `datetime` — An RFC 3339 datetime string (e.g., `"2026-03-15T14:00:00Z"`)
/// * `target_timezone` — An IANA timezone name (e.g., `"America/New_York"`)
///
/// # Returns
///
/// A [`ConvertedDatetime`] with the same instant expressed in the target timezone,
/// plus metadata (UTC offset, DST status).
///
/// # Errors
///
/// Returns [`TruthError::InvalidDatetime`] if the datetime string cannot be parsed,
/// or [`TruthError::InvalidTimezone`] if the timezone name is not a valid IANA timezone.
///
/// # Examples
///
/// ```
/// use truth_engine::temporal::convert_timezone;
///
/// let result = convert_timezone("2026-03-15T14:00:00Z", "America/New_York").unwrap();
/// assert_eq!(result.timezone, "America/New_York");
/// // March 15 2026 is EDT (UTC-4), so 14:00 UTC = 10:00 local
/// assert!(result.local.contains("10:00:00"));
/// ```
pub fn convert_timezone(
    datetime: &str,
    target_timezone: &str,
) -> Result<ConvertedDatetime, TruthError> {
    let dt = parse_rfc3339(datetime)?;
    let tz = parse_timezone(target_timezone)?;

    let local = dt.with_timezone(&tz);

    // Determine DST: compare the timezone's standard offset with the current offset.
    // If they differ, DST is active.
    let dst_active = is_dst_active(&local, &tz);

    let utc_offset = format_utc_offset(&local);

    Ok(ConvertedDatetime {
        utc: dt.to_rfc3339(),
        local: local.to_rfc3339(),
        timezone: target_timezone.to_string(),
        utc_offset,
        dst_active,
    })
}

// ── compute_duration ────────────────────────────────────────────────────────

/// Duration information between two timestamps.
#[derive(Debug, Clone, Serialize)]
pub struct DurationInfo {
    /// Total duration in seconds (negative if end is before start).
    pub total_seconds: i64,
    /// Days component of the decomposed duration.
    pub days: i64,
    /// Hours component (0-23).
    pub hours: i64,
    /// Minutes component (0-59).
    pub minutes: i64,
    /// Seconds component (0-59).
    pub seconds: i64,
    /// Human-readable representation (e.g., "2 days, 3 hours, 15 minutes").
    pub human_readable: String,
}

/// Compute the duration between two timestamps.
///
/// # Arguments
///
/// * `start` — An RFC 3339 datetime string
/// * `end` — An RFC 3339 datetime string
///
/// # Returns
///
/// A [`DurationInfo`] with the total seconds and decomposed days/hours/minutes/seconds.
/// If `end` is before `start`, `total_seconds` is negative and the decomposition
/// represents the absolute duration.
///
/// # Errors
///
/// Returns [`TruthError::InvalidDatetime`] if either datetime string cannot be parsed.
pub fn compute_duration(start: &str, end: &str) -> Result<DurationInfo, TruthError> {
    let start_dt = parse_rfc3339(start)?;
    let end_dt = parse_rfc3339(end)?;

    let total_seconds = (end_dt - start_dt).num_seconds();
    let abs_seconds = total_seconds.unsigned_abs();

    let days = (abs_seconds / 86400) as i64;
    let remainder = abs_seconds % 86400;
    let hours = (remainder / 3600) as i64;
    let remainder = remainder % 3600;
    let minutes = (remainder / 60) as i64;
    let seconds = (remainder % 60) as i64;

    let human_readable = format_human_duration(days, hours, minutes, seconds);

    Ok(DurationInfo {
        total_seconds,
        days,
        hours,
        minutes,
        seconds,
        human_readable,
    })
}

// ── adjust_timestamp ────────────────────────────────────────────────────────

/// The result of adjusting a timestamp by a duration.
#[derive(Debug, Clone, Serialize)]
pub struct AdjustedTimestamp {
    /// The original datetime (echoed back).
    pub original: String,
    /// The adjusted datetime in UTC (RFC 3339).
    pub adjusted_utc: String,
    /// The adjusted datetime in the source timezone (RFC 3339 with offset).
    pub adjusted_local: String,
    /// The normalized adjustment applied (e.g., "+2h30m").
    pub adjustment_applied: String,
}

/// Parsed duration components from an adjustment string.
#[derive(Debug, Clone, Default)]
struct ParsedDuration {
    sign: i64, // +1 or -1
    weeks: i64,
    days: i64,
    hours: i64,
    minutes: i64,
    seconds: i64,
}

/// Adjust a timestamp by adding or subtracting a duration.
///
/// # Arguments
///
/// * `datetime` — An RFC 3339 datetime string
/// * `adjustment` — A duration string (e.g., `"+2h"`, `"-30m"`, `"+1d2h30m"`, `"+1w"`)
/// * `timezone` — An IANA timezone name for interpreting day-level adjustments
///
/// # Duration Format
///
/// Must start with `+` or `-`, followed by one or more components:
/// - `Nw` — weeks
/// - `Nd` — days (timezone-aware: same wall-clock time, not +24h across DST)
/// - `Nh` — hours
/// - `Nm` — minutes
/// - `Ns` — seconds
///
/// Components can be combined: `+1d2h30m`, `-2w3d`.
///
/// # Errors
///
/// Returns [`TruthError::InvalidDatetime`] if the datetime cannot be parsed,
/// [`TruthError::InvalidTimezone`] if the timezone is invalid, or
/// [`TruthError::InvalidDuration`] if the adjustment string cannot be parsed.
pub fn adjust_timestamp(
    datetime: &str,
    adjustment: &str,
    timezone: &str,
) -> Result<AdjustedTimestamp, TruthError> {
    let dt = parse_rfc3339(datetime)?;
    let tz = parse_timezone(timezone)?;
    let parsed = parse_duration_string(adjustment)?;

    // For day/week adjustments, we work in local time to preserve wall-clock time
    // across DST transitions. For sub-day adjustments, we work in UTC.
    let local = dt.with_timezone(&tz);

    let adjusted_local = if parsed.weeks != 0 || parsed.days != 0 {
        // Day-level: adjust date in local time, then add sub-day components in UTC
        let total_days = parsed.sign * (parsed.weeks * 7 + parsed.days);
        let new_date = local.date_naive() + chrono::Duration::days(total_days);
        let new_local_naive = new_date.and_time(local.time());

        let adjusted_local_dt = tz
            .from_local_datetime(&new_local_naive)
            .single()
            .ok_or_else(|| {
                TruthError::InvalidDatetime(
                    "ambiguous or nonexistent local time after day adjustment".to_string(),
                )
            })?;

        // Add sub-day components in UTC
        let sub_day_seconds =
            parsed.sign * (parsed.hours * 3600 + parsed.minutes * 60 + parsed.seconds);
        adjusted_local_dt + chrono::Duration::seconds(sub_day_seconds)
    } else {
        // Sub-day only: simple UTC arithmetic
        let total_seconds =
            parsed.sign * (parsed.hours * 3600 + parsed.minutes * 60 + parsed.seconds);
        local + chrono::Duration::seconds(total_seconds)
    };

    let adjusted_utc = adjusted_local.with_timezone(&Utc);
    let normalized = normalize_duration_string(&parsed);

    Ok(AdjustedTimestamp {
        original: datetime.to_string(),
        adjusted_utc: adjusted_utc.to_rfc3339(),
        adjusted_local: adjusted_local.to_rfc3339(),
        adjustment_applied: normalized,
    })
}

// ── resolve_relative ────────────────────────────────────────────────────────

/// The result of resolving a relative time expression.
#[derive(Debug, Clone, Serialize)]
pub struct ResolvedDatetime {
    /// The resolved datetime in UTC (RFC 3339).
    pub resolved_utc: String,
    /// The resolved datetime in the given timezone (RFC 3339 with offset).
    pub resolved_local: String,
    /// The IANA timezone used for resolution.
    pub timezone: String,
    /// Human-readable interpretation (e.g., "Tuesday, February 24, 2026 at 2:00 PM EST").
    pub interpretation: String,
}

/// Resolve a relative time expression to an absolute datetime.
///
/// Uses ISO 8601 week start (Monday). For configurable week start, use
/// [`resolve_relative_with_options`].
///
/// # Arguments
///
/// * `anchor` — The reference "now" instant (typically `Utc::now()`)
/// * `expression` — A time expression (see [`resolve_relative_with_options`] for grammar)
/// * `timezone` — An IANA timezone name for interpreting local-time expressions
///
/// # Errors
///
/// Returns [`TruthError::InvalidExpression`] if the expression cannot be parsed
/// deterministically.
pub fn resolve_relative(
    anchor: DateTime<Utc>,
    expression: &str,
    timezone: &str,
) -> Result<ResolvedDatetime, TruthError> {
    resolve_relative_with_options(anchor, expression, timezone, &ResolveOptions::default())
}

/// Resolve a relative time expression to an absolute datetime with options.
///
/// # Arguments
///
/// * `anchor` — The reference "now" instant (typically `Utc::now()`)
/// * `expression` — A time expression (see grammar below)
/// * `timezone` — An IANA timezone name for interpreting local-time expressions
/// * `options` — Resolution options (week start day, etc.)
///
/// # Supported Expressions
///
/// **Anchored**: `"now"`, `"today"`, `"tomorrow"`, `"yesterday"`
///
/// **Weekday-relative**: `"next Monday"`, `"this Friday"`, `"last Wednesday"`
///
/// **Time-of-day**: `"morning"` (09:00), `"noon"` (12:00), `"afternoon"` (13:00),
/// `"evening"` (18:00), `"night"` (21:00), `"midnight"` (00:00),
/// `"end of day"` / `"eob"` (17:00), `"start of business"` / `"sob"` (09:00), `"lunch"` (12:00)
///
/// **Explicit time**: `"2pm"`, `"2:30pm"`, `"14:00"`, `"14:30"`
///
/// **Offset durations**: `"+2h"`, `"-30m"`, `"in 2 hours"`, `"30 minutes ago"`,
/// `"a week from now"`
///
/// **Combined**: `"next Tuesday at 2pm"`, `"tomorrow morning"`,
/// `"next Friday at 10:30am"`
///
/// **Period boundaries**: `"start of week"`, `"end of month"`, `"start of quarter"`,
/// `"next week"`, `"last month"`, `"next year"`
///
/// **Compound periods**: `"start of last week"`, `"end of next month"`,
/// `"start of next quarter"`, `"end of last year"`
///
/// **Ordinal dates**: `"first Monday of March"`, `"last Friday of the month"`,
/// `"third Tuesday of March 2026"`
///
/// **Passthrough**: Any valid RFC 3339 or ISO 8601 date string
///
/// # Errors
///
/// Returns [`TruthError::InvalidExpression`] if the expression cannot be parsed
/// deterministically. This function **never guesses** — it returns an error for
/// any ambiguous input.
pub fn resolve_relative_with_options(
    anchor: DateTime<Utc>,
    expression: &str,
    timezone: &str,
    options: &ResolveOptions,
) -> Result<ResolvedDatetime, TruthError> {
    let tz = parse_timezone(timezone)?;
    let local_anchor = anchor.with_timezone(&tz);
    let ws = options.week_start;

    // Normalize: trim, lowercase, strip articles
    let normalized = normalize_expression(expression);

    // Try each parser in order of specificity
    let resolved_local = try_passthrough_rfc3339(&normalized)
        .map(|dt| dt.with_timezone(&tz))
        .or_else(|| try_passthrough_iso_date(&normalized, &tz))
        .or_else(|| try_anchored(&normalized, &local_anchor, &tz))
        .or_else(|| try_combined_weekday_time(&normalized, &local_anchor, &tz))
        .or_else(|| try_combined_anchor_time(&normalized, &local_anchor, &tz))
        .or_else(|| try_weekday_relative(&normalized, &local_anchor, &tz))
        .or_else(|| try_compound_period(&normalized, &local_anchor, &tz, ws))
        .or_else(|| try_period_boundary(&normalized, &local_anchor, &tz, ws))
        .or_else(|| try_period_relative(&normalized, &local_anchor, &tz, ws))
        .or_else(|| try_ordinal_date(&normalized, &local_anchor, &tz))
        .or_else(|| try_natural_offset(&normalized, &anchor))
        .or_else(|| try_duration_offset(&normalized, &anchor))
        .or_else(|| try_time_of_day_named(&normalized, &local_anchor, &tz))
        .or_else(|| try_explicit_time(&normalized, &local_anchor, &tz))
        .ok_or_else(|| {
            TruthError::InvalidExpression(format!(
                "cannot parse expression: '{}'",
                expression.trim()
            ))
        })?;

    let resolved_utc = resolved_local.with_timezone(&Utc);
    let interpretation = format_interpretation(&resolved_local);

    Ok(ResolvedDatetime {
        resolved_utc: resolved_utc.to_rfc3339(),
        resolved_local: resolved_local.to_rfc3339(),
        timezone: timezone.to_string(),
        interpretation,
    })
}

// ── Internal helpers ────────────────────────────────────────────────────────

/// Parse an RFC 3339 datetime string into `DateTime<Utc>`.
fn parse_rfc3339(s: &str) -> Result<DateTime<Utc>, TruthError> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| TruthError::InvalidDatetime(format!("'{}': {}", s, e)))
}

/// Parse an IANA timezone string into `Tz`.
fn parse_timezone(s: &str) -> Result<Tz, TruthError> {
    s.parse::<Tz>()
        .map_err(|_| TruthError::InvalidTimezone(format!("'{}'", s)))
}

/// Determine if DST is active for a datetime in a timezone.
fn is_dst_active<T: TimeZone>(dt: &DateTime<T>, tz: &Tz) -> bool {
    // Compare January 1 offset (winter / standard) with the current offset.
    // If they differ, DST is active.
    let utc = dt.with_timezone(&Utc);
    let year = utc.year();

    let jan1 = Utc
        .with_ymd_and_hms(year, 1, 1, 12, 0, 0)
        .single()
        .unwrap_or(utc);
    let jan1_local = jan1.with_timezone(tz);

    let current_offset = dt.offset().fix().local_minus_utc();
    let jan_offset = jan1_local.offset().fix().local_minus_utc();

    current_offset != jan_offset
}

/// Format the UTC offset as a string (e.g., "-05:00", "+09:00").
fn format_utc_offset<T: TimeZone>(dt: &DateTime<T>) -> String {
    let offset_secs = dt.offset().fix().local_minus_utc();
    let sign = if offset_secs >= 0 { "+" } else { "-" };
    let abs_secs = offset_secs.unsigned_abs();
    let hours = abs_secs / 3600;
    let minutes = (abs_secs % 3600) / 60;
    format!("{sign}{hours:02}:{minutes:02}")
}

/// Format a human-readable duration string.
fn format_human_duration(days: i64, hours: i64, minutes: i64, seconds: i64) -> String {
    let mut parts = Vec::new();
    if days > 0 {
        parts.push(format!("{} day{}", days, if days == 1 { "" } else { "s" }));
    }
    if hours > 0 {
        parts.push(format!(
            "{} hour{}",
            hours,
            if hours == 1 { "" } else { "s" }
        ));
    }
    if minutes > 0 {
        parts.push(format!(
            "{} minute{}",
            minutes,
            if minutes == 1 { "" } else { "s" }
        ));
    }
    if seconds > 0 || parts.is_empty() {
        parts.push(format!(
            "{} second{}",
            seconds,
            if seconds == 1 { "" } else { "s" }
        ));
    }
    parts.join(", ")
}

/// Parse a duration adjustment string (e.g., "+2h", "-1d30m", "+1w2d").
fn parse_duration_string(s: &str) -> Result<ParsedDuration, TruthError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(TruthError::InvalidDuration("empty duration".to_string()));
    }

    let (sign, rest) = match s.as_bytes().first() {
        Some(b'+') => (1i64, &s[1..]),
        Some(b'-') => (-1i64, &s[1..]),
        _ => {
            return Err(TruthError::InvalidDuration(format!(
                "duration must start with '+' or '-': '{s}'"
            )));
        }
    };

    if rest.is_empty() {
        return Err(TruthError::InvalidDuration(format!(
            "duration has no components: '{s}'"
        )));
    }

    let mut parsed = ParsedDuration {
        sign,
        ..Default::default()
    };

    let mut num_buf = String::new();
    let mut found_any = false;

    for ch in rest.chars() {
        if ch.is_ascii_digit() {
            num_buf.push(ch);
        } else {
            if num_buf.is_empty() {
                return Err(TruthError::InvalidDuration(format!(
                    "expected number before '{ch}' in '{s}'"
                )));
            }
            let n: i64 = num_buf
                .parse()
                .map_err(|_| TruthError::InvalidDuration(format!("invalid number in '{s}'")))?;
            num_buf.clear();
            found_any = true;

            match ch {
                'w' | 'W' => parsed.weeks += n,
                'd' | 'D' => parsed.days += n,
                'h' | 'H' => parsed.hours += n,
                'm' | 'M' => parsed.minutes += n,
                's' | 'S' => parsed.seconds += n,
                _ => {
                    return Err(TruthError::InvalidDuration(format!(
                        "unknown unit '{ch}' in '{s}'"
                    )));
                }
            }
        }
    }

    // Trailing number without unit
    if !num_buf.is_empty() {
        return Err(TruthError::InvalidDuration(format!(
            "number without unit at end of '{s}'"
        )));
    }

    if !found_any {
        return Err(TruthError::InvalidDuration(format!(
            "no valid components in '{s}'"
        )));
    }

    Ok(parsed)
}

/// Normalize a parsed duration back to a string like "+1d2h30m".
fn normalize_duration_string(d: &ParsedDuration) -> String {
    let sign = if d.sign >= 0 { "+" } else { "-" };
    let mut parts = String::from(sign);
    if d.weeks != 0 {
        parts.push_str(&format!("{}w", d.weeks));
    }
    if d.days != 0 {
        parts.push_str(&format!("{}d", d.days));
    }
    if d.hours != 0 {
        parts.push_str(&format!("{}h", d.hours));
    }
    if d.minutes != 0 {
        parts.push_str(&format!("{}m", d.minutes));
    }
    if d.seconds != 0 {
        parts.push_str(&format!("{}s", d.seconds));
    }
    if parts.len() == 1 {
        // Only sign, no components (shouldn't happen after parsing, but defensive)
        parts.push_str("0s");
    }
    parts
}

// ── resolve_relative expression parsers ─────────────────────────────────────

/// Normalize expression: trim, lowercase, strip common articles (but not "a"/"an" at start
/// since those are meaningful for patterns like "a week from now").
fn normalize_expression(s: &str) -> String {
    let s = s.trim().to_lowercase();
    // Strip articles in the middle: "the", "a", "an"
    let s = s
        .replace(" the ", " ")
        .replace(" a ", " ")
        .replace(" an ", " ");
    // Strip leading "the " only (not "a "/"an " — they matter for "a week from now")
    let s = s.strip_prefix("the ").unwrap_or(&s).to_string();
    // Collapse multiple spaces
    let mut result = String::new();
    let mut prev_space = false;
    for ch in s.chars() {
        if ch == ' ' {
            if !prev_space {
                result.push(' ');
            }
            prev_space = true;
        } else {
            result.push(ch);
            prev_space = false;
        }
    }
    result.trim().to_string()
}

/// Try to parse as an RFC 3339 passthrough.
fn try_passthrough_rfc3339(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .ok()
}

/// Try to parse as an ISO 8601 date (YYYY-MM-DD) → start of day in timezone.
fn try_passthrough_iso_date(s: &str, tz: &Tz) -> Option<DateTime<Tz>> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .ok()
        .and_then(|date| {
            let naive = date.and_hms_opt(0, 0, 0)?;
            tz.from_local_datetime(&naive).single()
        })
}

/// Try anchored references: "now", "today", "tomorrow", "yesterday".
fn try_anchored(s: &str, local: &DateTime<Tz>, tz: &Tz) -> Option<DateTime<Tz>> {
    match s {
        "now" => Some(*local),
        "today" => make_local_start_of_day(local, tz),
        "tomorrow" => {
            let next = local.date_naive().succ_opt()?;
            let naive = next.and_hms_opt(0, 0, 0)?;
            tz.from_local_datetime(&naive).single()
        }
        "yesterday" => {
            let prev = local.date_naive().pred_opt()?;
            let naive = prev.and_hms_opt(0, 0, 0)?;
            tz.from_local_datetime(&naive).single()
        }
        _ => None,
    }
}

/// Try weekday-relative: "next Monday", "this Friday", "last Wednesday".
fn try_weekday_relative(s: &str, local: &DateTime<Tz>, tz: &Tz) -> Option<DateTime<Tz>> {
    let parts: Vec<&str> = s.splitn(2, ' ').collect();
    if parts.len() != 2 {
        return None;
    }

    let modifier = parts[0];
    let weekday = parse_weekday(parts[1])?;
    let current = local.weekday();

    let target_date = match modifier {
        "next" => {
            // Always future: if today is the same weekday, go to next week
            let days_ahead =
                (weekday.num_days_from_monday() as i64 - current.num_days_from_monday() as i64 + 7)
                    % 7;
            let days_ahead = if days_ahead == 0 { 7 } else { days_ahead };
            local.date_naive() + chrono::Duration::days(days_ahead)
        }
        "this" => {
            // Same week: may be past or future
            let diff =
                weekday.num_days_from_monday() as i64 - current.num_days_from_monday() as i64;
            local.date_naive() + chrono::Duration::days(diff)
        }
        "last" => {
            // Always past: if today is the same weekday, go to last week
            let days_back =
                (current.num_days_from_monday() as i64 - weekday.num_days_from_monday() as i64 + 7)
                    % 7;
            let days_back = if days_back == 0 { 7 } else { days_back };
            local.date_naive() - chrono::Duration::days(days_back)
        }
        _ => return None,
    };

    let naive = target_date.and_hms_opt(0, 0, 0)?;
    tz.from_local_datetime(&naive).single()
}

/// Try combined weekday + time: "next Tuesday at 2pm", "next Friday at 10:30am".
fn try_combined_weekday_time(s: &str, local: &DateTime<Tz>, tz: &Tz) -> Option<DateTime<Tz>> {
    // Pattern: "(next|this|last) <weekday> at <time>"
    // or: "(next|this|last) <weekday> <named_time>"
    let parts: Vec<&str> = s.splitn(3, ' ').collect();
    if parts.len() < 2 {
        return None;
    }

    let modifier = parts[0];
    if !matches!(modifier, "next" | "this" | "last") {
        return None;
    }

    // Check for weekday in parts[1]
    let weekday_str = parts[1];
    let _weekday = parse_weekday(weekday_str)?;

    // Get the base date from weekday-relative
    let weekday_expr = format!("{} {}", modifier, weekday_str);
    let base = try_weekday_relative(&weekday_expr, local, tz)?;

    if parts.len() == 2 {
        return Some(base);
    }

    let time_part = parts[2];

    // Handle "at <time>" pattern
    if let Some(at_time) = time_part.strip_prefix("at ") {
        let time = parse_time_string(at_time)?;
        let naive = base.date_naive().and_time(time);
        return tz.from_local_datetime(&naive).single();
    }

    // Handle named time: "morning", "afternoon", etc.
    if let Some(time) = named_time_to_naive(time_part) {
        let naive = base.date_naive().and_time(time);
        return tz.from_local_datetime(&naive).single();
    }

    None
}

/// Try combined anchor + time: "tomorrow at 2pm", "today at noon", "tomorrow morning".
fn try_combined_anchor_time(s: &str, local: &DateTime<Tz>, tz: &Tz) -> Option<DateTime<Tz>> {
    let parts: Vec<&str> = s.splitn(2, ' ').collect();
    if parts.len() != 2 {
        return None;
    }

    let anchor_str = parts[0];
    if !matches!(anchor_str, "today" | "tomorrow" | "yesterday") {
        return None;
    }

    let base = try_anchored(anchor_str, local, tz)?;
    let time_part = parts[1];

    // "at <time>" — try named time first (e.g., "at noon"), then explicit time (e.g., "at 2pm")
    if let Some(at_time) = time_part.strip_prefix("at ") {
        if let Some(time) = named_time_to_naive(at_time) {
            let naive = base.date_naive().and_time(time);
            return tz.from_local_datetime(&naive).single();
        }
        let time = parse_time_string(at_time)?;
        let naive = base.date_naive().and_time(time);
        return tz.from_local_datetime(&naive).single();
    }

    // Named time
    if let Some(time) = named_time_to_naive(time_part) {
        let naive = base.date_naive().and_time(time);
        return tz.from_local_datetime(&naive).single();
    }

    None
}

/// Try time-of-day named anchors: "morning", "noon", "afternoon", etc.
fn try_time_of_day_named(s: &str, local: &DateTime<Tz>, tz: &Tz) -> Option<DateTime<Tz>> {
    let time = named_time_to_naive(s)?;
    let naive = local.date_naive().and_time(time);
    tz.from_local_datetime(&naive).single()
}

/// Try explicit time: "2pm", "2:30pm", "14:00".
fn try_explicit_time(s: &str, local: &DateTime<Tz>, tz: &Tz) -> Option<DateTime<Tz>> {
    let time = parse_time_string(s)?;
    let naive = local.date_naive().and_time(time);
    tz.from_local_datetime(&naive).single()
}

/// Try natural offset: "in 2 hours", "30 minutes ago", "a week from now".
fn try_natural_offset(s: &str, anchor: &DateTime<Utc>) -> Option<DateTime<Tz>> {
    // "in N unit(s)"
    if let Some(rest) = s.strip_prefix("in ") {
        let (n, unit) = parse_natural_number_and_unit(rest)?;
        let seconds = unit_to_seconds(n, &unit)?;
        let result = *anchor + chrono::Duration::seconds(seconds);
        // Return as UTC (which is a valid Tz via chrono_tz)
        let utc_tz: Tz = "UTC".parse().ok()?;
        return Some(result.with_timezone(&utc_tz));
    }

    // "N unit(s) ago"
    if s.ends_with(" ago") {
        let rest = s.strip_suffix(" ago")?;
        let (n, unit) = parse_natural_number_and_unit(rest)?;
        let seconds = unit_to_seconds(n, &unit)?;
        let result = *anchor - chrono::Duration::seconds(seconds);
        let utc_tz: Tz = "UTC".parse().ok()?;
        return Some(result.with_timezone(&utc_tz));
    }

    // "a/an <unit> from now"
    if s.ends_with(" from now") {
        let rest = s.strip_suffix(" from now")?;
        let (n, unit) = parse_natural_number_and_unit_with_article(rest)?;
        let seconds = unit_to_seconds(n, &unit)?;
        let result = *anchor + chrono::Duration::seconds(seconds);
        let utc_tz: Tz = "UTC".parse().ok()?;
        return Some(result.with_timezone(&utc_tz));
    }

    None
}

/// Try duration offset: "+2h", "-30m", "+1d2h30m".
fn try_duration_offset(s: &str, anchor: &DateTime<Utc>) -> Option<DateTime<Tz>> {
    if !s.starts_with('+') && !s.starts_with('-') {
        return None;
    }
    let parsed = parse_duration_string(s).ok()?;
    let total_seconds = parsed.sign
        * (parsed.weeks * 7 * 86400
            + parsed.days * 86400
            + parsed.hours * 3600
            + parsed.minutes * 60
            + parsed.seconds);
    let result = *anchor + chrono::Duration::seconds(total_seconds);
    let utc_tz: Tz = "UTC".parse().ok()?;
    Some(result.with_timezone(&utc_tz))
}

/// Try period boundary: "start of week", "end of month", etc.
fn try_period_boundary(
    s: &str,
    local: &DateTime<Tz>,
    tz: &Tz,
    ws: WeekStartDay,
) -> Option<DateTime<Tz>> {
    match s {
        "start of today" => make_local_start_of_day(local, tz),
        "end of today" => {
            let naive = local.date_naive().and_hms_opt(23, 59, 59)?;
            tz.from_local_datetime(&naive).single()
        }
        "start of week" => {
            let days_since_start = days_from_week_start(local.weekday(), ws);
            let start = local.date_naive() - chrono::Duration::days(days_since_start);
            let naive = start.and_hms_opt(0, 0, 0)?;
            tz.from_local_datetime(&naive).single()
        }
        "end of week" => {
            let days_until_end = 6 - days_from_week_start(local.weekday(), ws);
            let end = local.date_naive() + chrono::Duration::days(days_until_end);
            let naive = end.and_hms_opt(23, 59, 59)?;
            tz.from_local_datetime(&naive).single()
        }
        "start of month" => {
            let date = NaiveDate::from_ymd_opt(local.year(), local.month(), 1)?;
            let naive = date.and_hms_opt(0, 0, 0)?;
            tz.from_local_datetime(&naive).single()
        }
        "end of month" => {
            let (y, m) = if local.month() == 12 {
                (local.year() + 1, 1)
            } else {
                (local.year(), local.month() + 1)
            };
            let first_next = NaiveDate::from_ymd_opt(y, m, 1)?;
            let last_day = first_next.pred_opt()?;
            let naive = last_day.and_hms_opt(23, 59, 59)?;
            tz.from_local_datetime(&naive).single()
        }
        "start of year" => {
            let date = NaiveDate::from_ymd_opt(local.year(), 1, 1)?;
            let naive = date.and_hms_opt(0, 0, 0)?;
            tz.from_local_datetime(&naive).single()
        }
        "end of year" => {
            let date = NaiveDate::from_ymd_opt(local.year(), 12, 31)?;
            let naive = date.and_hms_opt(23, 59, 59)?;
            tz.from_local_datetime(&naive).single()
        }
        "start of quarter" => {
            let q_start_month = ((local.month() - 1) / 3) * 3 + 1;
            let date = NaiveDate::from_ymd_opt(local.year(), q_start_month, 1)?;
            let naive = date.and_hms_opt(0, 0, 0)?;
            tz.from_local_datetime(&naive).single()
        }
        "end of quarter" => {
            let q_end_month = ((local.month() - 1) / 3 + 1) * 3;
            let (y, m) = if q_end_month == 12 {
                (local.year() + 1, 1)
            } else {
                (local.year(), q_end_month + 1)
            };
            let first_next = NaiveDate::from_ymd_opt(y, m, 1)?;
            let last_day = first_next.pred_opt()?;
            let naive = last_day.and_hms_opt(23, 59, 59)?;
            tz.from_local_datetime(&naive).single()
        }
        _ => None,
    }
}

/// Try period relative: "next week", "last month", "next year", etc.
fn try_period_relative(
    s: &str,
    local: &DateTime<Tz>,
    tz: &Tz,
    ws: WeekStartDay,
) -> Option<DateTime<Tz>> {
    match s {
        "next week" => {
            let days_until_next_start = 7 - days_from_week_start(local.weekday(), ws);
            let start = local.date_naive() + chrono::Duration::days(days_until_next_start);
            let naive = start.and_hms_opt(0, 0, 0)?;
            tz.from_local_datetime(&naive).single()
        }
        "last week" => {
            let days_since_start = days_from_week_start(local.weekday(), ws);
            let this_start = local.date_naive() - chrono::Duration::days(days_since_start);
            let last_start = this_start - chrono::Duration::days(7);
            let naive = last_start.and_hms_opt(0, 0, 0)?;
            tz.from_local_datetime(&naive).single()
        }
        "next month" => {
            let (y, m) = if local.month() == 12 {
                (local.year() + 1, 1)
            } else {
                (local.year(), local.month() + 1)
            };
            let date = NaiveDate::from_ymd_opt(y, m, 1)?;
            let naive = date.and_hms_opt(0, 0, 0)?;
            tz.from_local_datetime(&naive).single()
        }
        "last month" => {
            let (y, m) = if local.month() == 1 {
                (local.year() - 1, 12)
            } else {
                (local.year(), local.month() - 1)
            };
            let date = NaiveDate::from_ymd_opt(y, m, 1)?;
            let naive = date.and_hms_opt(0, 0, 0)?;
            tz.from_local_datetime(&naive).single()
        }
        "next year" => {
            let date = NaiveDate::from_ymd_opt(local.year() + 1, 1, 1)?;
            let naive = date.and_hms_opt(0, 0, 0)?;
            tz.from_local_datetime(&naive).single()
        }
        "last year" => {
            let date = NaiveDate::from_ymd_opt(local.year() - 1, 1, 1)?;
            let naive = date.and_hms_opt(0, 0, 0)?;
            tz.from_local_datetime(&naive).single()
        }
        _ => None,
    }
}

/// Try compound period: "start of last week", "end of next month", etc.
///
/// Combines a boundary (start/end) with a period relative (last/next week/month/year/quarter).
fn try_compound_period(
    s: &str,
    local: &DateTime<Tz>,
    tz: &Tz,
    ws: WeekStartDay,
) -> Option<DateTime<Tz>> {
    let (is_start, rest) = if let Some(r) = s.strip_prefix("start of ") {
        (true, r)
    } else if let Some(r) = s.strip_prefix("end of ") {
        (false, r)
    } else {
        return None;
    };

    match rest {
        "last week" => {
            let days_since_start = days_from_week_start(local.weekday(), ws);
            let this_start = local.date_naive() - chrono::Duration::days(days_since_start);
            let last_start = this_start - chrono::Duration::days(7);
            if is_start {
                let naive = last_start.and_hms_opt(0, 0, 0)?;
                tz.from_local_datetime(&naive).single()
            } else {
                let last_end = last_start + chrono::Duration::days(6);
                let naive = last_end.and_hms_opt(23, 59, 59)?;
                tz.from_local_datetime(&naive).single()
            }
        }
        "next week" => {
            let days_until_next_start = 7 - days_from_week_start(local.weekday(), ws);
            let next_start = local.date_naive() + chrono::Duration::days(days_until_next_start);
            if is_start {
                let naive = next_start.and_hms_opt(0, 0, 0)?;
                tz.from_local_datetime(&naive).single()
            } else {
                let next_end = next_start + chrono::Duration::days(6);
                let naive = next_end.and_hms_opt(23, 59, 59)?;
                tz.from_local_datetime(&naive).single()
            }
        }
        "last month" => {
            let (y, m) = if local.month() == 1 {
                (local.year() - 1, 12)
            } else {
                (local.year(), local.month() - 1)
            };
            if is_start {
                let date = NaiveDate::from_ymd_opt(y, m, 1)?;
                let naive = date.and_hms_opt(0, 0, 0)?;
                tz.from_local_datetime(&naive).single()
            } else {
                // Last day of prev month = day before 1st of current month
                let first_current = NaiveDate::from_ymd_opt(local.year(), local.month(), 1)?;
                let last_day = first_current.pred_opt()?;
                let naive = last_day.and_hms_opt(23, 59, 59)?;
                tz.from_local_datetime(&naive).single()
            }
        }
        "next month" => {
            let (y, m) = if local.month() == 12 {
                (local.year() + 1, 1)
            } else {
                (local.year(), local.month() + 1)
            };
            if is_start {
                let date = NaiveDate::from_ymd_opt(y, m, 1)?;
                let naive = date.and_hms_opt(0, 0, 0)?;
                tz.from_local_datetime(&naive).single()
            } else {
                // Last day of next month
                let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
                let first_after = NaiveDate::from_ymd_opt(ny, nm, 1)?;
                let last_day = first_after.pred_opt()?;
                let naive = last_day.and_hms_opt(23, 59, 59)?;
                tz.from_local_datetime(&naive).single()
            }
        }
        "last year" => {
            let y = local.year() - 1;
            if is_start {
                let date = NaiveDate::from_ymd_opt(y, 1, 1)?;
                let naive = date.and_hms_opt(0, 0, 0)?;
                tz.from_local_datetime(&naive).single()
            } else {
                let date = NaiveDate::from_ymd_opt(y, 12, 31)?;
                let naive = date.and_hms_opt(23, 59, 59)?;
                tz.from_local_datetime(&naive).single()
            }
        }
        "next year" => {
            let y = local.year() + 1;
            if is_start {
                let date = NaiveDate::from_ymd_opt(y, 1, 1)?;
                let naive = date.and_hms_opt(0, 0, 0)?;
                tz.from_local_datetime(&naive).single()
            } else {
                let date = NaiveDate::from_ymd_opt(y, 12, 31)?;
                let naive = date.and_hms_opt(23, 59, 59)?;
                tz.from_local_datetime(&naive).single()
            }
        }
        "last quarter" => {
            let current_q = (local.month() - 1) / 3; // 0-based: Q1=0, Q2=1, Q3=2, Q4=3
            let (prev_y, prev_q) = if current_q == 0 {
                (local.year() - 1, 3)
            } else {
                (local.year(), current_q - 1)
            };
            let q_first_month = prev_q * 3 + 1;
            if is_start {
                let date = NaiveDate::from_ymd_opt(prev_y, q_first_month, 1)?;
                let naive = date.and_hms_opt(0, 0, 0)?;
                tz.from_local_datetime(&naive).single()
            } else {
                let q_last_month = prev_q * 3 + 3;
                let (ny, nm) = if q_last_month == 12 {
                    (prev_y + 1, 1)
                } else {
                    (prev_y, q_last_month + 1)
                };
                let first_after = NaiveDate::from_ymd_opt(ny, nm, 1)?;
                let last_day = first_after.pred_opt()?;
                let naive = last_day.and_hms_opt(23, 59, 59)?;
                tz.from_local_datetime(&naive).single()
            }
        }
        "next quarter" => {
            let current_q = (local.month() - 1) / 3;
            let (next_y, next_q) = if current_q == 3 {
                (local.year() + 1, 0)
            } else {
                (local.year(), current_q + 1)
            };
            let q_first_month = next_q * 3 + 1;
            if is_start {
                let date = NaiveDate::from_ymd_opt(next_y, q_first_month, 1)?;
                let naive = date.and_hms_opt(0, 0, 0)?;
                tz.from_local_datetime(&naive).single()
            } else {
                let q_last_month = next_q * 3 + 3;
                let (ny, nm) = if q_last_month == 12 {
                    (next_y + 1, 1)
                } else {
                    (next_y, q_last_month + 1)
                };
                let first_after = NaiveDate::from_ymd_opt(ny, nm, 1)?;
                let last_day = first_after.pred_opt()?;
                let naive = last_day.and_hms_opt(23, 59, 59)?;
                tz.from_local_datetime(&naive).single()
            }
        }
        _ => None,
    }
}

/// Try ordinal date: "first Monday of March", "last Friday of the month",
/// "third Tuesday of March 2026".
fn try_ordinal_date(s: &str, local: &DateTime<Tz>, tz: &Tz) -> Option<DateTime<Tz>> {
    // Pattern: "<ordinal> <weekday> of <month> [year]"
    // or: "last <weekday> of <month>" / "last day of <month>"
    let parts: Vec<&str> = s.split_whitespace().collect();

    if parts.len() < 4 || parts.iter().position(|&p| p == "of")? < 2 {
        return None;
    }

    let of_idx = parts.iter().position(|&p| p == "of")?;
    if of_idx < 2 {
        return None;
    }

    let ordinal_str = parts[0];
    let target_str = parts[1];

    // Parse "last day of <month>"
    if ordinal_str == "last" && target_str == "day" {
        let month_str = parts.get(of_idx + 1)?;
        let month = parse_month(month_str)?;
        let year = if let Some(y_str) = parts.get(of_idx + 2) {
            y_str.parse::<i32>().ok()?
        } else {
            local.year()
        };
        let (ny, nm) = if month == 12 {
            (year + 1, 1)
        } else {
            (year, month + 1)
        };
        let first_next = NaiveDate::from_ymd_opt(ny, nm, 1)?;
        let last_day = first_next.pred_opt()?;
        let naive = last_day.and_hms_opt(0, 0, 0)?;
        return tz.from_local_datetime(&naive).single();
    }

    let weekday = parse_weekday(target_str)?;

    let month_part = parts.get(of_idx + 1)?;
    // "the month" → current month, otherwise parse month name
    let (month, year) = if *month_part == "month" {
        (local.month(), local.year())
    } else if let Some(month_num) = parse_month(month_part) {
        let year = if let Some(y_str) = parts.get(of_idx + 2) {
            y_str.parse::<i32>().unwrap_or(local.year())
        } else {
            local.year()
        };
        (month_num, year)
    } else if *month_part == "next" && parts.get(of_idx + 2) == Some(&"month") {
        let (y, m) = if local.month() == 12 {
            (local.year() + 1, 1)
        } else {
            (local.year(), local.month() + 1)
        };
        (m, y)
    } else {
        return None;
    };

    let ordinal = parse_ordinal(ordinal_str)?;

    let date = find_nth_weekday_in_month(year, month, weekday, ordinal)?;
    let naive = date.and_hms_opt(0, 0, 0)?;
    tz.from_local_datetime(&naive).single()
}

/// Find the Nth weekday in a month. ordinal < 0 means "last" (-1), "second to last" (-2), etc.
fn find_nth_weekday_in_month(
    year: i32,
    month: u32,
    weekday: Weekday,
    ordinal: i32,
) -> Option<NaiveDate> {
    if ordinal > 0 {
        // Forward from the first of the month
        let first = NaiveDate::from_ymd_opt(year, month, 1)?;
        let first_wd = first.weekday();
        let diff = (weekday.num_days_from_monday() as i32 - first_wd.num_days_from_monday() as i32
            + 7)
            % 7;
        let first_occurrence = first + chrono::Duration::days(diff as i64);
        let target = first_occurrence + chrono::Duration::weeks((ordinal - 1) as i64);
        // Verify still in the same month
        if target.month() == month {
            Some(target)
        } else {
            None
        }
    } else {
        // Backward from the last of the month (ordinal = -1 means "last")
        let (ny, nm) = if month == 12 {
            (year + 1, 1)
        } else {
            (year, month + 1)
        };
        let first_next = NaiveDate::from_ymd_opt(ny, nm, 1)?;
        let last = first_next.pred_opt()?;
        let last_wd = last.weekday();
        let diff =
            (last_wd.num_days_from_monday() as i32 - weekday.num_days_from_monday() as i32 + 7) % 7;
        let last_occurrence = last - chrono::Duration::days(diff as i64);
        let target = last_occurrence - chrono::Duration::weeks((-ordinal - 1) as i64);
        // Verify still in the same month
        if target.month() == month {
            Some(target)
        } else {
            None
        }
    }
}

// ── Parsing helpers ─────────────────────────────────────────────────────────

/// Parse a weekday name (case-insensitive, supports full and abbreviated).
fn parse_weekday(s: &str) -> Option<Weekday> {
    match s {
        "monday" | "mon" => Some(Weekday::Mon),
        "tuesday" | "tue" | "tues" => Some(Weekday::Tue),
        "wednesday" | "wed" => Some(Weekday::Wed),
        "thursday" | "thu" | "thurs" => Some(Weekday::Thu),
        "friday" | "fri" => Some(Weekday::Fri),
        "saturday" | "sat" => Some(Weekday::Sat),
        "sunday" | "sun" => Some(Weekday::Sun),
        _ => None,
    }
}

/// Parse a month name to number (1-12).
fn parse_month(s: &str) -> Option<u32> {
    match s {
        "january" | "jan" => Some(1),
        "february" | "feb" => Some(2),
        "march" | "mar" => Some(3),
        "april" | "apr" => Some(4),
        "may" => Some(5),
        "june" | "jun" => Some(6),
        "july" | "jul" => Some(7),
        "august" | "aug" => Some(8),
        "september" | "sep" | "sept" => Some(9),
        "october" | "oct" => Some(10),
        "november" | "nov" => Some(11),
        "december" | "dec" => Some(12),
        _ => None,
    }
}

/// Parse an ordinal: "first"→1, "second"→2, ..., "last"→-1.
fn parse_ordinal(s: &str) -> Option<i32> {
    match s {
        "first" | "1st" => Some(1),
        "second" | "2nd" => Some(2),
        "third" | "3rd" => Some(3),
        "fourth" | "4th" => Some(4),
        "fifth" | "5th" => Some(5),
        "last" => Some(-1),
        _ => None,
    }
}

/// Map named time to NaiveTime.
fn named_time_to_naive(s: &str) -> Option<NaiveTime> {
    match s {
        "morning" | "start of business" | "sob" => NaiveTime::from_hms_opt(9, 0, 0),
        "noon" | "lunch" => NaiveTime::from_hms_opt(12, 0, 0),
        "afternoon" => NaiveTime::from_hms_opt(13, 0, 0),
        "end of day" | "end of business" | "eob" => NaiveTime::from_hms_opt(17, 0, 0),
        "evening" => NaiveTime::from_hms_opt(18, 0, 0),
        "night" => NaiveTime::from_hms_opt(21, 0, 0),
        "midnight" => NaiveTime::from_hms_opt(0, 0, 0),
        _ => None,
    }
}

/// Parse a time string: "2pm", "2:30pm", "14:00", "14:30:00".
fn parse_time_string(s: &str) -> Option<NaiveTime> {
    let s = s.trim();

    // 24-hour format: "14:00", "14:30", "14:30:00"
    if let Ok(t) = NaiveTime::parse_from_str(s, "%H:%M:%S") {
        return Some(t);
    }
    if let Ok(t) = NaiveTime::parse_from_str(s, "%H:%M") {
        return Some(t);
    }

    // 12-hour format: "2pm", "2:30pm", "2:30:00pm", "2 pm"
    let s_no_space = s.replace(' ', "");
    let (time_part, is_pm) = if s_no_space.ends_with("pm") {
        (s_no_space.strip_suffix("pm")?, true)
    } else if s_no_space.ends_with("am") {
        (s_no_space.strip_suffix("am")?, false)
    } else {
        return None;
    };

    let parts: Vec<&str> = time_part.split(':').collect();
    let hour: u32 = parts.first()?.parse().ok()?;
    let minute: u32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let second: u32 = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);

    let hour24 = match (hour, is_pm) {
        (12, true) => 12,
        (12, false) => 0,
        (h, true) => h + 12,
        (h, false) => h,
    };

    NaiveTime::from_hms_opt(hour24, minute, second)
}

/// Parse "N unit(s)" from natural language (e.g., "2 hours", "30 minutes").
fn parse_natural_number_and_unit(s: &str) -> Option<(i64, String)> {
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }
    let n: i64 = parts[0].parse().ok()?;
    let unit = normalize_time_unit(parts[1])?;
    Some((n, unit))
}

/// Parse "a/an unit from now" or "N unit(s) from now" prefix.
fn parse_natural_number_and_unit_with_article(s: &str) -> Option<(i64, String)> {
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    // "a week", "an hour"
    if parts[0] == "a" || parts[0] == "an" {
        if parts.len() < 2 {
            return None;
        }
        let unit = normalize_time_unit(parts[1])?;
        return Some((1, unit));
    }

    // "2 hours", "30 minutes"
    parse_natural_number_and_unit(s)
}

/// Normalize a time unit name to a standard form.
fn normalize_time_unit(s: &str) -> Option<String> {
    match s {
        "second" | "seconds" | "sec" | "secs" => Some("seconds".to_string()),
        "minute" | "minutes" | "min" | "mins" => Some("minutes".to_string()),
        "hour" | "hours" | "hr" | "hrs" => Some("hours".to_string()),
        "day" | "days" => Some("days".to_string()),
        "week" | "weeks" | "wk" | "wks" => Some("weeks".to_string()),
        _ => None,
    }
}

/// Convert a number and unit to total seconds.
fn unit_to_seconds(n: i64, unit: &str) -> Option<i64> {
    let multiplier = match unit {
        "seconds" => 1,
        "minutes" => 60,
        "hours" => 3600,
        "days" => 86400,
        "weeks" => 604800,
        _ => return None,
    };
    Some(n * multiplier)
}

/// Create a DateTime at the start of the day (00:00) in the given timezone.
fn make_local_start_of_day(local: &DateTime<Tz>, tz: &Tz) -> Option<DateTime<Tz>> {
    let naive = local.date_naive().and_hms_opt(0, 0, 0)?;
    tz.from_local_datetime(&naive).single()
}

/// Format a human-readable interpretation string.
fn format_interpretation<T: TimeZone>(dt: &DateTime<T>) -> String
where
    T::Offset: std::fmt::Display,
{
    dt.format("%A, %B %-d, %Y at %-I:%M %p %Z").to_string()
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    // ── convert_timezone tests ──────────────────────────────────────────

    #[test]
    fn test_convert_utc_to_eastern() {
        let result = convert_timezone("2026-03-15T14:00:00Z", "America/New_York").unwrap();
        assert_eq!(result.timezone, "America/New_York");
        // March 15 2026 is EDT (UTC-4), so 14:00 UTC = 10:00 local
        assert!(result.local.contains("10:00:00"));
        assert_eq!(result.utc, "2026-03-15T14:00:00+00:00");
    }

    #[test]
    fn test_convert_eastern_to_pacific() {
        // Input is in UTC-5 (EST), convert to Pacific
        let result = convert_timezone("2026-01-15T14:00:00-05:00", "America/Los_Angeles").unwrap();
        assert_eq!(result.timezone, "America/Los_Angeles");
        // Jan 15 is PST (UTC-8). The input is 14:00 EST = 19:00 UTC = 11:00 PST
        assert!(result.local.contains("11:00:00"));
    }

    #[test]
    fn test_convert_across_dst_spring_forward() {
        // March 8, 2026: US spring forward (2:00 AM → 3:00 AM)
        // Before DST: Jan 15, 2026 — EST (UTC-5)
        let winter = convert_timezone("2026-01-15T12:00:00Z", "America/New_York").unwrap();
        assert_eq!(winter.utc_offset, "-05:00");
        assert!(!winter.dst_active);

        // After DST: March 15, 2026 — EDT (UTC-4)
        let summer = convert_timezone("2026-03-15T12:00:00Z", "America/New_York").unwrap();
        assert_eq!(summer.utc_offset, "-04:00");
        assert!(summer.dst_active);
    }

    #[test]
    fn test_convert_across_dst_fall_back() {
        // November 1, 2026: US fall back (2:00 AM → 1:00 AM)
        // After fall back: Nov 2 — EST (UTC-5)
        let result = convert_timezone("2026-11-02T12:00:00Z", "America/New_York").unwrap();
        assert_eq!(result.utc_offset, "-05:00");
        assert!(!result.dst_active);
    }

    #[test]
    fn test_convert_utc_offset_correct() {
        let result = convert_timezone("2026-06-15T12:00:00Z", "Asia/Tokyo").unwrap();
        assert_eq!(result.utc_offset, "+09:00");
        assert!(!result.dst_active); // Japan does not observe DST
    }

    #[test]
    fn test_convert_dst_active_flag() {
        // Summer in New York — DST active
        let summer = convert_timezone("2026-07-15T12:00:00Z", "America/New_York").unwrap();
        assert!(summer.dst_active);

        // Winter in New York — DST not active
        let winter = convert_timezone("2026-12-15T12:00:00Z", "America/New_York").unwrap();
        assert!(!winter.dst_active);
    }

    #[test]
    fn test_convert_invalid_timezone_returns_error() {
        let result = convert_timezone("2026-03-15T14:00:00Z", "Invalid/Zone");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Invalid timezone"), "got: {err}");
    }

    #[test]
    fn test_convert_invalid_datetime_returns_error() {
        let result = convert_timezone("not-a-datetime", "America/New_York");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Invalid datetime"), "got: {err}");
    }

    // ── compute_duration tests ──────────────────────────────────────────

    #[test]
    fn test_duration_same_day() {
        let result = compute_duration("2026-03-16T09:00:00Z", "2026-03-16T17:00:00Z").unwrap();
        assert_eq!(result.total_seconds, 28800); // 8 hours
        assert_eq!(result.hours, 8);
        assert_eq!(result.days, 0);
        assert_eq!(result.minutes, 0);
    }

    #[test]
    fn test_duration_across_days() {
        let result = compute_duration(
            "2026-03-13T17:00:00Z", // Friday 5pm
            "2026-03-16T09:00:00Z", // Monday 9am
        )
        .unwrap();
        assert_eq!(result.total_seconds, 230400); // 2d + 16h = 2*86400 + 16*3600
        assert_eq!(result.days, 2);
        assert_eq!(result.hours, 16);
    }

    #[test]
    fn test_duration_negative_direction() {
        let result = compute_duration("2026-03-16T17:00:00Z", "2026-03-16T09:00:00Z").unwrap();
        assert_eq!(result.total_seconds, -28800);
        // Decomposition is always positive
        assert_eq!(result.hours, 8);
    }

    #[test]
    fn test_duration_exact_days() {
        let result = compute_duration("2026-03-16T00:00:00Z", "2026-03-19T00:00:00Z").unwrap();
        assert_eq!(result.days, 3);
        assert_eq!(result.hours, 0);
        assert_eq!(result.minutes, 0);
        assert_eq!(result.seconds, 0);
    }

    #[test]
    fn test_duration_sub_minute() {
        let result = compute_duration("2026-03-16T10:00:00Z", "2026-03-16T10:00:45Z").unwrap();
        assert_eq!(result.total_seconds, 45);
        assert_eq!(result.seconds, 45);
        assert_eq!(result.minutes, 0);
    }

    #[test]
    fn test_duration_human_readable_format() {
        let result = compute_duration("2026-03-16T00:00:00Z", "2026-03-18T03:15:00Z").unwrap();
        assert_eq!(result.human_readable, "2 days, 3 hours, 15 minutes");
    }

    #[test]
    fn test_duration_invalid_input() {
        let result = compute_duration("not-a-datetime", "2026-03-16T10:00:00Z");
        assert!(result.is_err());
    }

    // ── adjust_timestamp tests ──────────────────────────────────────────

    #[test]
    fn test_adjust_add_hours() {
        let result = adjust_timestamp("2026-03-16T10:00:00Z", "+2h", "UTC").unwrap();
        assert!(result.adjusted_utc.contains("12:00:00"));
    }

    #[test]
    fn test_adjust_subtract_days() {
        let result = adjust_timestamp("2026-03-05T10:00:00Z", "-3d", "UTC").unwrap();
        assert!(result.adjusted_utc.contains("2026-03-02"));
    }

    #[test]
    fn test_adjust_add_minutes() {
        let result = adjust_timestamp("2026-03-16T10:00:00Z", "+90m", "UTC").unwrap();
        assert!(result.adjusted_utc.contains("11:30:00"));
    }

    #[test]
    fn test_adjust_add_weeks() {
        let result = adjust_timestamp("2026-03-02T10:00:00Z", "+2w", "UTC").unwrap();
        assert!(result.adjusted_utc.contains("2026-03-16"));
    }

    #[test]
    fn test_adjust_compound_duration() {
        let result = adjust_timestamp("2026-03-16T10:00:00Z", "+1d2h30m", "UTC").unwrap();
        // March 16 10:00 + 1d2h30m = March 17 12:30
        assert!(result.adjusted_utc.contains("2026-03-17"));
        assert!(result.adjusted_utc.contains("12:30:00"));
    }

    #[test]
    fn test_adjust_day_across_dst() {
        // March 8 2026: US spring forward. +1d should preserve wall-clock time.
        let result = adjust_timestamp(
            "2026-03-07T22:00:00-05:00", // 10pm EST (= 03:00 UTC on March 8)
            "+1d",
            "America/New_York",
        )
        .unwrap();
        // March 8, 10pm EDT (now in EDT = -04:00)
        assert!(result.adjusted_local.contains("22:00:00"));
    }

    #[test]
    fn test_adjust_negative_compound() {
        let result = adjust_timestamp("2026-03-16T10:00:00Z", "-1d12h", "UTC").unwrap();
        // March 16 10:00 - 1d12h = March 14 22:00
        assert!(result.adjusted_utc.contains("2026-03-14"));
        assert!(result.adjusted_utc.contains("22:00:00"));
    }

    #[test]
    fn test_adjust_add_seconds() {
        let result = adjust_timestamp("2026-03-16T10:00:00Z", "+3600s", "UTC").unwrap();
        assert!(result.adjusted_utc.contains("11:00:00"));
    }

    #[test]
    fn test_adjust_invalid_format() {
        let result = adjust_timestamp("2026-03-16T10:00:00Z", "2h", "UTC");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("must start with '+' or '-'"), "got: {err}");
    }

    #[test]
    fn test_adjust_zero_duration() {
        let result = adjust_timestamp("2026-03-16T10:00:00Z", "+0h", "UTC").unwrap();
        assert!(result.adjusted_utc.contains("10:00:00"));
    }

    // ── resolve_relative tests ──────────────────────────────────────────

    fn anchor() -> DateTime<Utc> {
        // Wednesday, February 18, 2026, 14:30:00 UTC
        Utc.with_ymd_and_hms(2026, 2, 18, 14, 30, 0).unwrap()
    }

    #[test]
    fn test_resolve_now() {
        let result = resolve_relative(anchor(), "now", "UTC").unwrap();
        assert!(result.resolved_utc.contains("14:30:00"));
    }

    #[test]
    fn test_resolve_today() {
        let result = resolve_relative(anchor(), "today", "UTC").unwrap();
        assert!(result.resolved_utc.contains("2026-02-18"));
        assert!(result.resolved_utc.contains("00:00:00"));
    }

    #[test]
    fn test_resolve_tomorrow() {
        let result = resolve_relative(anchor(), "tomorrow", "UTC").unwrap();
        assert!(result.resolved_utc.contains("2026-02-19"));
        assert!(result.resolved_utc.contains("00:00:00"));
    }

    #[test]
    fn test_resolve_yesterday() {
        let result = resolve_relative(anchor(), "yesterday", "UTC").unwrap();
        assert!(result.resolved_utc.contains("2026-02-17"));
    }

    #[test]
    fn test_resolve_next_monday_from_wednesday() {
        // Anchor is Wednesday Feb 18 → next Monday is Feb 23
        let result = resolve_relative(anchor(), "next Monday", "UTC").unwrap();
        assert!(result.resolved_utc.contains("2026-02-23"));
    }

    #[test]
    fn test_resolve_next_friday_from_friday() {
        // If anchor is Friday Feb 20 → next Friday should be Feb 27 (not same day)
        let fri_anchor = Utc.with_ymd_and_hms(2026, 2, 20, 10, 0, 0).unwrap();
        let result = resolve_relative(fri_anchor, "next Friday", "UTC").unwrap();
        assert!(result.resolved_utc.contains("2026-02-27"));
    }

    #[test]
    fn test_resolve_this_wednesday_from_monday() {
        let mon_anchor = Utc.with_ymd_and_hms(2026, 2, 16, 10, 0, 0).unwrap();
        let result = resolve_relative(mon_anchor, "this Wednesday", "UTC").unwrap();
        assert!(result.resolved_utc.contains("2026-02-18"));
    }

    #[test]
    fn test_resolve_last_tuesday_from_thursday() {
        let thu_anchor = Utc.with_ymd_and_hms(2026, 2, 19, 10, 0, 0).unwrap();
        let result = resolve_relative(thu_anchor, "last Tuesday", "UTC").unwrap();
        assert!(result.resolved_utc.contains("2026-02-17"));
    }

    #[test]
    fn test_resolve_morning() {
        let result = resolve_relative(anchor(), "morning", "UTC").unwrap();
        assert!(result.resolved_utc.contains("09:00:00"));
    }

    #[test]
    fn test_resolve_noon() {
        let result = resolve_relative(anchor(), "noon", "UTC").unwrap();
        assert!(result.resolved_utc.contains("12:00:00"));
    }

    #[test]
    fn test_resolve_afternoon() {
        let result = resolve_relative(anchor(), "afternoon", "UTC").unwrap();
        assert!(result.resolved_utc.contains("13:00:00"));
    }

    #[test]
    fn test_resolve_evening() {
        let result = resolve_relative(anchor(), "evening", "UTC").unwrap();
        assert!(result.resolved_utc.contains("18:00:00"));
    }

    #[test]
    fn test_resolve_eob() {
        let result = resolve_relative(anchor(), "eob", "UTC").unwrap();
        assert!(result.resolved_utc.contains("17:00:00"));
    }

    #[test]
    fn test_resolve_midnight() {
        let result = resolve_relative(anchor(), "midnight", "UTC").unwrap();
        assert!(result.resolved_utc.contains("00:00:00"));
    }

    #[test]
    fn test_resolve_2pm() {
        let result = resolve_relative(anchor(), "2pm", "UTC").unwrap();
        assert!(result.resolved_utc.contains("14:00:00"));
    }

    #[test]
    fn test_resolve_2_30pm() {
        let result = resolve_relative(anchor(), "2:30pm", "UTC").unwrap();
        assert!(result.resolved_utc.contains("14:30:00"));
    }

    #[test]
    fn test_resolve_14_00() {
        let result = resolve_relative(anchor(), "14:00", "UTC").unwrap();
        assert!(result.resolved_utc.contains("14:00:00"));
    }

    #[test]
    fn test_resolve_in_2_hours() {
        let result = resolve_relative(anchor(), "in 2 hours", "UTC").unwrap();
        assert!(result.resolved_utc.contains("16:30:00"));
    }

    #[test]
    fn test_resolve_30_minutes_ago() {
        let result = resolve_relative(anchor(), "30 minutes ago", "UTC").unwrap();
        assert!(result.resolved_utc.contains("14:00:00"));
    }

    #[test]
    fn test_resolve_in_3_days() {
        let result = resolve_relative(anchor(), "in 3 days", "UTC").unwrap();
        assert!(result.resolved_utc.contains("2026-02-21"));
    }

    #[test]
    fn test_resolve_a_week_from_now() {
        let result = resolve_relative(anchor(), "a week from now", "UTC").unwrap();
        assert!(result.resolved_utc.contains("2026-02-25"));
    }

    #[test]
    fn test_resolve_next_tuesday_at_2pm() {
        // Anchor is Wed Feb 18 → next Tuesday is Feb 24, at 2pm
        let result = resolve_relative(anchor(), "next Tuesday at 2pm", "UTC").unwrap();
        assert!(result.resolved_utc.contains("2026-02-24"));
        assert!(result.resolved_utc.contains("14:00:00"));
    }

    #[test]
    fn test_resolve_tomorrow_at_10_30am() {
        let result = resolve_relative(anchor(), "tomorrow at 10:30am", "UTC").unwrap();
        assert!(result.resolved_utc.contains("2026-02-19"));
        assert!(result.resolved_utc.contains("10:30:00"));
    }

    #[test]
    fn test_resolve_tomorrow_morning() {
        let result = resolve_relative(anchor(), "tomorrow morning", "UTC").unwrap();
        assert!(result.resolved_utc.contains("2026-02-19"));
        assert!(result.resolved_utc.contains("09:00:00"));
    }

    #[test]
    fn test_resolve_next_friday_evening() {
        // Anchor is Wed Feb 18 → next Friday is Feb 20, evening = 18:00
        let result = resolve_relative(anchor(), "next Friday evening", "UTC").unwrap();
        assert!(result.resolved_utc.contains("2026-02-20"));
        assert!(result.resolved_utc.contains("18:00:00"));
    }

    #[test]
    fn test_resolve_today_at_noon() {
        let result = resolve_relative(anchor(), "today at noon", "UTC").unwrap();
        assert!(result.resolved_utc.contains("2026-02-18"));
        assert!(result.resolved_utc.contains("12:00:00"));
    }

    #[test]
    fn test_resolve_start_of_week() {
        // Anchor is Wed Feb 18 → start of ISO week is Mon Feb 16
        let result = resolve_relative(anchor(), "start of week", "UTC").unwrap();
        assert!(result.resolved_utc.contains("2026-02-16"));
        assert!(result.resolved_utc.contains("00:00:00"));
    }

    #[test]
    fn test_resolve_end_of_month() {
        let result = resolve_relative(anchor(), "end of month", "UTC").unwrap();
        assert!(result.resolved_utc.contains("2026-02-28"));
        assert!(result.resolved_utc.contains("23:59:59"));
    }

    #[test]
    fn test_resolve_start_of_quarter() {
        // Feb is Q1, so start of quarter is Jan 1
        let result = resolve_relative(anchor(), "start of quarter", "UTC").unwrap();
        assert!(result.resolved_utc.contains("2026-01-01"));
    }

    #[test]
    fn test_resolve_next_week() {
        // Anchor is Wed Feb 18 → next Monday is Feb 23
        let result = resolve_relative(anchor(), "next week", "UTC").unwrap();
        assert!(result.resolved_utc.contains("2026-02-23"));
    }

    #[test]
    fn test_resolve_next_month() {
        let result = resolve_relative(anchor(), "next month", "UTC").unwrap();
        assert!(result.resolved_utc.contains("2026-03-01"));
    }

    #[test]
    fn test_resolve_first_monday_of_march() {
        let result = resolve_relative(anchor(), "first Monday of March", "UTC").unwrap();
        // March 2026: first Monday is March 2
        assert!(result.resolved_utc.contains("2026-03-02"));
    }

    #[test]
    fn test_resolve_last_friday_of_month() {
        let result = resolve_relative(anchor(), "last Friday of the month", "UTC").unwrap();
        // February 2026: last Friday is Feb 27
        assert!(result.resolved_utc.contains("2026-02-27"));
    }

    #[test]
    fn test_resolve_third_tuesday_of_march_2026() {
        let result = resolve_relative(anchor(), "third Tuesday of March 2026", "UTC").unwrap();
        // March 2026: 1st Tue=3, 2nd=10, 3rd=17
        assert!(result.resolved_utc.contains("2026-03-17"));
    }

    #[test]
    fn test_resolve_passthrough_rfc3339() {
        let input = "2026-06-15T10:00:00-04:00";
        let result = resolve_relative(anchor(), input, "UTC").unwrap();
        // Should preserve the instant (convert to UTC)
        assert!(result.resolved_utc.contains("2026-06-15"));
        assert!(result.resolved_utc.contains("14:00:00"));
    }

    #[test]
    fn test_resolve_passthrough_iso_date() {
        let result = resolve_relative(anchor(), "2026-03-15", "America/New_York").unwrap();
        // Should be start of day March 15 in Eastern time
        assert!(result.resolved_local.contains("2026-03-15"));
        assert!(result.resolved_local.contains("00:00:00"));
    }

    #[test]
    fn test_resolve_case_insensitive() {
        let result = resolve_relative(anchor(), "Next TUESDAY at 2PM", "UTC").unwrap();
        assert!(result.resolved_utc.contains("2026-02-24"));
        assert!(result.resolved_utc.contains("14:00:00"));
    }

    #[test]
    fn test_resolve_articles_ignored() {
        let result = resolve_relative(anchor(), "a week from now", "UTC").unwrap();
        assert!(result.resolved_utc.contains("2026-02-25"));
    }

    #[test]
    fn test_resolve_unparseable_returns_error() {
        let result = resolve_relative(anchor(), "gobbledygook", "UTC");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("cannot parse expression"), "got: {err}");
    }

    #[test]
    fn test_resolve_interpretation_format() {
        let result = resolve_relative(anchor(), "next Tuesday at 2pm", "UTC").unwrap();
        // Should contain day of week and date
        assert!(result.interpretation.contains("Tuesday"));
        assert!(result.interpretation.contains("February 24"));
        assert!(result.interpretation.contains("2026"));
    }

    // ── Compound period expression tests ────────────────────────────────

    #[test]
    fn test_resolve_start_of_last_week() {
        // Anchor is Wed Feb 18 → last week started Mon Feb 9
        let result = resolve_relative(anchor(), "start of last week", "UTC").unwrap();
        assert!(result.resolved_utc.contains("2026-02-09"));
        assert!(result.resolved_utc.contains("00:00:00"));
    }

    #[test]
    fn test_resolve_end_of_last_week() {
        // Anchor is Wed Feb 18 → last week ended Sun Feb 15
        let result = resolve_relative(anchor(), "end of last week", "UTC").unwrap();
        assert!(result.resolved_utc.contains("2026-02-15"));
        assert!(result.resolved_utc.contains("23:59:59"));
    }

    #[test]
    fn test_resolve_start_of_next_week() {
        // Anchor is Wed Feb 18 → next week starts Mon Feb 23
        let result = resolve_relative(anchor(), "start of next week", "UTC").unwrap();
        assert!(result.resolved_utc.contains("2026-02-23"));
        assert!(result.resolved_utc.contains("00:00:00"));
    }

    #[test]
    fn test_resolve_end_of_next_week() {
        // Anchor is Wed Feb 18 → next week ends Sun Mar 1
        let result = resolve_relative(anchor(), "end of next week", "UTC").unwrap();
        assert!(result.resolved_utc.contains("2026-03-01"));
        assert!(result.resolved_utc.contains("23:59:59"));
    }

    #[test]
    fn test_resolve_start_of_last_month() {
        let result = resolve_relative(anchor(), "start of last month", "UTC").unwrap();
        assert!(result.resolved_utc.contains("2026-01-01"));
        assert!(result.resolved_utc.contains("00:00:00"));
    }

    #[test]
    fn test_resolve_end_of_last_month() {
        // Jan has 31 days
        let result = resolve_relative(anchor(), "end of last month", "UTC").unwrap();
        assert!(result.resolved_utc.contains("2026-01-31"));
        assert!(result.resolved_utc.contains("23:59:59"));
    }

    #[test]
    fn test_resolve_start_of_next_month() {
        let result = resolve_relative(anchor(), "start of next month", "UTC").unwrap();
        assert!(result.resolved_utc.contains("2026-03-01"));
        assert!(result.resolved_utc.contains("00:00:00"));
    }

    #[test]
    fn test_resolve_end_of_next_month() {
        // March has 31 days
        let result = resolve_relative(anchor(), "end of next month", "UTC").unwrap();
        assert!(result.resolved_utc.contains("2026-03-31"));
        assert!(result.resolved_utc.contains("23:59:59"));
    }

    #[test]
    fn test_resolve_start_of_next_year() {
        let result = resolve_relative(anchor(), "start of next year", "UTC").unwrap();
        assert!(result.resolved_utc.contains("2027-01-01"));
        assert!(result.resolved_utc.contains("00:00:00"));
    }

    #[test]
    fn test_resolve_end_of_last_quarter() {
        // Anchor is Feb 2026 (Q1) → last quarter is Q4 2025 → ends Dec 31, 2025
        let result = resolve_relative(anchor(), "end of last quarter", "UTC").unwrap();
        assert!(result.resolved_utc.contains("2025-12-31"));
        assert!(result.resolved_utc.contains("23:59:59"));
    }

    // ── Sunday week start tests ─────────────────────────────────────────

    #[test]
    fn test_resolve_start_of_week_sunday() {
        // Anchor is Wed Feb 18 → with Sunday start, week started Sun Feb 15
        let options = ResolveOptions {
            week_start: WeekStartDay::Sunday,
        };
        let result =
            resolve_relative_with_options(anchor(), "start of week", "UTC", &options).unwrap();
        assert!(result.resolved_utc.contains("2026-02-15"));
        assert!(result.resolved_utc.contains("00:00:00"));
    }

    #[test]
    fn test_resolve_end_of_week_sunday() {
        // Anchor is Wed Feb 18 → with Sunday start, week ends Sat Feb 21
        let options = ResolveOptions {
            week_start: WeekStartDay::Sunday,
        };
        let result =
            resolve_relative_with_options(anchor(), "end of week", "UTC", &options).unwrap();
        assert!(result.resolved_utc.contains("2026-02-21"));
        assert!(result.resolved_utc.contains("23:59:59"));
    }

    #[test]
    fn test_resolve_start_of_last_week_sunday() {
        // Anchor is Wed Feb 18 → with Sunday start, last week started Sun Feb 8
        let options = ResolveOptions {
            week_start: WeekStartDay::Sunday,
        };
        let result =
            resolve_relative_with_options(anchor(), "start of last week", "UTC", &options).unwrap();
        assert!(result.resolved_utc.contains("2026-02-08"));
        assert!(result.resolved_utc.contains("00:00:00"));
    }

    #[test]
    fn test_resolve_next_week_sunday() {
        // Anchor is Wed Feb 18 → with Sunday start, next week starts Sun Feb 22
        let options = ResolveOptions {
            week_start: WeekStartDay::Sunday,
        };
        let result = resolve_relative_with_options(anchor(), "next week", "UTC", &options).unwrap();
        assert!(result.resolved_utc.contains("2026-02-22"));
        assert!(result.resolved_utc.contains("00:00:00"));
    }
}
