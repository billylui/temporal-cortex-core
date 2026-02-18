//! # temporal-cortex-toon-python
//!
//! Python bindings for the TOON format encoder/decoder and truth-engine,
//! built with PyO3.
//!
//! Exposes the following functions to Python as the `temporal_cortex_toon` module:
//!
//! - `encode(json)` -- JSON string -> TOON string
//! - `decode(toon)` -- TOON string -> JSON string
//! - `filter_and_encode(json, patterns)` -- semantic filter + TOON encode
//! - `expand_rrule(...)` -- RRULE expansion -> JSON string of events

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Encode a JSON string into TOON format.
///
/// Args:
///     json: A valid JSON string.
///
/// Returns:
///     The TOON-encoded string.
///
/// Raises:
///     ValueError: If the input is not valid JSON or encoding fails.
#[pyfunction]
fn encode(json: &str) -> PyResult<String> {
    toon_core::encode(json).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Decode a TOON string back into JSON.
///
/// Args:
///     toon: A valid TOON string.
///
/// Returns:
///     The JSON string.
///
/// Raises:
///     ValueError: If the input is not valid TOON or decoding fails.
#[pyfunction]
fn decode(toon: &str) -> PyResult<String> {
    toon_core::decode(toon).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Filter fields from a JSON string by pattern, then encode to TOON.
///
/// Patterns support dot-separated paths and wildcards:
/// - `"etag"` -- strip the top-level field named "etag"
/// - `"items.etag"` -- strip "etag" inside objects under "items"
/// - `"*.etag"` -- wildcard: strip "etag" at any depth
///
/// Args:
///     json: A valid JSON string.
///     patterns: A list of field patterns to strip.
///
/// Returns:
///     The filtered TOON-encoded string.
///
/// Raises:
///     ValueError: If the input is not valid JSON or encoding fails.
#[pyfunction]
fn filter_and_encode(json: &str, patterns: Vec<String>) -> PyResult<String> {
    let pattern_refs: Vec<&str> = patterns.iter().map(|s| s.as_str()).collect();
    toon_core::filter_and_encode(json, &pattern_refs)
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Expand an RRULE into concrete event instances, returned as a JSON string.
///
/// Each event in the returned JSON array has `start` and `end` fields
/// as ISO 8601 UTC datetime strings.
///
/// Args:
///     rrule: RFC 5545 RRULE string (e.g., "FREQ=DAILY;COUNT=3").
///     dtstart: Local datetime string (e.g., "2026-02-17T14:00:00").
///     duration_minutes: Duration of each event instance in minutes.
///     timezone: IANA timezone identifier (e.g., "America/Los_Angeles").
///     until: Optional end boundary for expansion (local datetime string).
///     max_count: Optional maximum number of instances to generate.
///
/// Returns:
///     A JSON string containing an array of event objects with `start` and `end` fields.
///
/// Raises:
///     ValueError: If the RRULE or timezone is invalid.
#[pyfunction]
#[pyo3(signature = (rrule, dtstart, duration_minutes, timezone, until=None, max_count=None))]
fn expand_rrule(
    rrule: &str,
    dtstart: &str,
    duration_minutes: i64,
    timezone: &str,
    until: Option<&str>,
    max_count: Option<u32>,
) -> PyResult<String> {
    let events = truth_engine::expand_rrule(
        rrule,
        dtstart,
        duration_minutes as u32,
        timezone,
        until,
        max_count,
    )
    .map_err(|e| PyValueError::new_err(e.to_string()))?;

    // Serialize to JSON: [{"start": "...", "end": "..."}, ...]
    let json_events: Vec<serde_json::Value> = events
        .into_iter()
        .map(|evt| {
            serde_json::json!({
                "start": evt.start.to_rfc3339(),
                "end": evt.end.to_rfc3339(),
            })
        })
        .collect();

    serde_json::to_string(&json_events).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Merge N event streams into unified availability within a time window.
///
/// Args:
///     streams_json: JSON array of stream objects, each with `stream_id` (str) and
///         `events` (array of `{start, end}` objects with ISO 8601 strings).
///     window_start: Start of the time window (ISO 8601 datetime string).
///     window_end: End of the time window (ISO 8601 datetime string).
///     opaque: If True, hide source counts in busy blocks (privacy mode). Default: True.
///
/// Returns:
///     A JSON string with `{busy, free, window_start, window_end, privacy}`.
///
/// Raises:
///     ValueError: If the JSON input is malformed or datetimes are invalid.
#[pyfunction]
#[pyo3(signature = (streams_json, window_start, window_end, opaque=true))]
fn merge_availability(
    streams_json: &str,
    window_start: &str,
    window_end: &str,
    opaque: bool,
) -> PyResult<String> {
    use chrono::{DateTime, NaiveDateTime, Utc};
    use truth_engine::availability::{EventStream, PrivacyLevel};
    use truth_engine::expander::ExpandedEvent;

    #[derive(serde::Deserialize)]
    struct StreamInput {
        stream_id: String,
        events: Vec<EventInput>,
    }
    #[derive(serde::Deserialize)]
    struct EventInput {
        start: String,
        end: String,
    }

    fn parse_dt(s: &str) -> PyResult<DateTime<Utc>> {
        if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
            return Ok(dt.with_timezone(&Utc));
        }
        NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
            .map(|ndt| ndt.and_utc())
            .map_err(|e| PyValueError::new_err(format!("Invalid datetime '{}': {}", s, e)))
    }

    let inputs: Vec<StreamInput> = serde_json::from_str(streams_json)
        .map_err(|e| PyValueError::new_err(format!("Invalid streams JSON: {}", e)))?;

    let ws = parse_dt(window_start)?;
    let we = parse_dt(window_end)?;

    let privacy = if opaque {
        PrivacyLevel::Opaque
    } else {
        PrivacyLevel::Full
    };

    let streams: Vec<EventStream> = inputs
        .into_iter()
        .map(|si| {
            let events: PyResult<Vec<ExpandedEvent>> = si
                .events
                .into_iter()
                .map(|ei| {
                    let start = parse_dt(&ei.start)?;
                    let end = parse_dt(&ei.end)?;
                    Ok(ExpandedEvent { start, end })
                })
                .collect();
            Ok(EventStream {
                stream_id: si.stream_id,
                events: events?,
            })
        })
        .collect::<PyResult<Vec<_>>>()?;

    let result = truth_engine::merge_availability(&streams, ws, we, privacy);

    serde_json::to_string(&result)
        .map_err(|e| PyValueError::new_err(format!("Serialization error: {}", e)))
}

/// Find the first free slot of at least `min_duration_minutes` across N merged
/// event streams.
///
/// Args:
///     streams_json: JSON array of stream objects (same format as merge_availability).
///     window_start: Start of the search window (ISO 8601 datetime string).
///     window_end: End of the search window (ISO 8601 datetime string).
///     min_duration_minutes: Minimum free slot duration in minutes.
///
/// Returns:
///     A JSON string with `{start, end, duration_minutes}` or `"null"` if no slot found.
///
/// Raises:
///     ValueError: If the JSON input is malformed or datetimes are invalid.
#[pyfunction]
fn find_first_free_across(
    streams_json: &str,
    window_start: &str,
    window_end: &str,
    min_duration_minutes: i64,
) -> PyResult<String> {
    use chrono::{DateTime, NaiveDateTime, Utc};
    use truth_engine::availability::EventStream;
    use truth_engine::expander::ExpandedEvent;

    #[derive(serde::Deserialize)]
    struct StreamInput {
        stream_id: String,
        events: Vec<EventInput>,
    }
    #[derive(serde::Deserialize)]
    struct EventInput {
        start: String,
        end: String,
    }

    fn parse_dt(s: &str) -> PyResult<DateTime<Utc>> {
        if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
            return Ok(dt.with_timezone(&Utc));
        }
        NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
            .map(|ndt| ndt.and_utc())
            .map_err(|e| PyValueError::new_err(format!("Invalid datetime '{}': {}", s, e)))
    }

    let inputs: Vec<StreamInput> = serde_json::from_str(streams_json)
        .map_err(|e| PyValueError::new_err(format!("Invalid streams JSON: {}", e)))?;

    let ws = parse_dt(window_start)?;
    let we = parse_dt(window_end)?;

    let streams: Vec<EventStream> = inputs
        .into_iter()
        .map(|si| {
            let events: PyResult<Vec<ExpandedEvent>> = si
                .events
                .into_iter()
                .map(|ei| {
                    let start = parse_dt(&ei.start)?;
                    let end = parse_dt(&ei.end)?;
                    Ok(ExpandedEvent { start, end })
                })
                .collect();
            Ok(EventStream {
                stream_id: si.stream_id,
                events: events?,
            })
        })
        .collect::<PyResult<Vec<_>>>()?;

    let slot = truth_engine::find_first_free_across(&streams, ws, we, min_duration_minutes);

    match slot {
        Some(s) => serde_json::to_string(&s)
            .map_err(|e| PyValueError::new_err(format!("Serialization error: {}", e))),
        None => Ok("null".to_string()),
    }
}

/// The `temporal_cortex_toon` Python module, implemented in Rust via PyO3.
#[pymodule]
fn temporal_cortex_toon(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(encode, m)?)?;
    m.add_function(wrap_pyfunction!(decode, m)?)?;
    m.add_function(wrap_pyfunction!(filter_and_encode, m)?)?;
    m.add_function(wrap_pyfunction!(expand_rrule, m)?)?;
    m.add_function(wrap_pyfunction!(merge_availability, m)?)?;
    m.add_function(wrap_pyfunction!(find_first_free_across, m)?)?;
    Ok(())
}
