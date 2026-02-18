//! Semantic filtering -- strip unnecessary fields before TOON encoding.
//!
//! This module provides pattern-based field stripping for JSON values,
//! enabling significant size reduction when encoding to TOON by removing
//! API noise fields (etags, internal IDs, redundant links, etc.).
//!
//! # Pattern syntax
//!
//! - `"etag"` -- strip the top-level field named "etag"
//! - `"items.etag"` -- strip "etag" inside objects under "items"
//! - `"*.etag"` -- wildcard: strip "etag" at any depth
//! - `"attendees.*.responseStatus"` -- strip "responseStatus" inside each
//!   array element of "attendees"

use crate::error::Result;
use serde_json::{Map, Value};

/// A parsed filter pattern, split on dots for efficient matching.
///
/// Each segment is either a literal field name or the wildcard `*`.
/// For example, `"items.*.etag"` becomes `["items", "*", "etag"]`.
#[derive(Debug, Clone)]
struct Pattern<'a> {
    segments: Vec<&'a str>,
}

impl<'a> Pattern<'a> {
    /// Parse a dot-separated pattern string into segments.
    fn parse(pattern: &'a str) -> Self {
        Self {
            segments: pattern.split('.').collect(),
        }
    }
}

/// Strip fields from a JSON value according to the given patterns.
///
/// Returns a new `Value` with matching fields removed. The function
/// recursively walks objects and arrays, applying pattern matching at
/// each level.
///
/// # Pattern syntax
///
/// - `"field"` -- remove top-level key
/// - `"parent.child"` -- remove `child` inside `parent`
/// - `"*.field"` -- remove `field` at any nesting depth
/// - `"arr.*.field"` -- remove `field` inside each element of array `arr`
///
/// # Examples
///
/// ```
/// use serde_json::json;
/// use toon_core::filter_fields;
///
/// let value = json!({"name": "Alice", "etag": "abc", "kind": "event"});
/// let filtered = filter_fields(&value, &["etag", "kind"]);
/// assert_eq!(filtered, json!({"name": "Alice"}));
/// ```
pub fn filter_fields(value: &Value, patterns: &[&str]) -> Value {
    if patterns.is_empty() {
        return value.clone();
    }
    let parsed: Vec<Pattern<'_>> = patterns.iter().map(|p| Pattern::parse(p)).collect();
    apply_filter(value, &parsed)
}

/// Internal recursive filter engine.
///
/// Walks the value tree, applying all active patterns at the current depth.
/// For each object key, patterns are checked in three ways:
///
/// 1. **Terminal match**: a single-segment pattern matching the key name
///    causes the key to be removed entirely.
/// 2. **Path descent**: a multi-segment pattern whose first segment matches
///    the key name descends into the child with the remaining segments.
/// 3. **Wildcard propagation**: patterns starting with `*` both try to match
///    the current key (via the remaining segments) AND propagate the full
///    wildcard pattern into children for matching at deeper levels.
///
/// Arrays are transparent to pattern matching: all patterns pass through
/// to each array element unchanged.
fn apply_filter(value: &Value, patterns: &[Pattern<'_>]) -> Value {
    match value {
        Value::Object(map) => filter_object(map, patterns),
        Value::Array(arr) => filter_array(arr, patterns),
        // Primitives (string, number, bool, null) are returned as-is.
        other => other.clone(),
    }
}

/// Filter an object map by removing keys that match terminal patterns,
/// and recursing into children with narrowed patterns.
fn filter_object(map: &Map<String, Value>, patterns: &[Pattern<'_>]) -> Value {
    let mut result = Map::new();

    for (key, child) in map {
        // Determine whether this key should be removed and collect
        // the set of patterns to propagate into the child value.
        let mut remove = false;
        let mut child_patterns: Vec<Pattern<'_>> = Vec::new();

        for pattern in patterns {
            let segs = &pattern.segments;
            if segs.is_empty() {
                continue;
            }

            let first = segs[0];
            let rest = &segs[1..];

            if first == "*" {
                // Wildcard: `*` matches any single key at this level.
                if rest.is_empty() {
                    // Pattern is just `*` -- remove every key (unusual but valid).
                    remove = true;
                    break;
                }
                // The wildcard consumed one level. Check if the remaining
                // pattern's first segment matches this key as a terminal.
                if rest.len() == 1 && rest[0] == key {
                    // e.g. pattern `*.etag` and key is `etag` -- remove it.
                    remove = true;
                    break;
                }
                // Otherwise, narrow the rest as a child pattern if the next
                // segment matches this key or is another wildcard.
                if rest[0] == key || rest[0] == "*" {
                    // Descend with segments after the matched key.
                    child_patterns.push(Pattern {
                        segments: rest[1..].to_vec(),
                    });
                }
                // Always propagate the full wildcard pattern into children
                // so it can match at deeper levels too.
                child_patterns.push(pattern.clone());
            } else if first == key {
                // Literal match on the first segment.
                if rest.is_empty() {
                    // Terminal match: `"etag"` matches key "etag" -- remove.
                    remove = true;
                    break;
                }
                // Multi-segment: descend with the remaining path.
                child_patterns.push(Pattern {
                    segments: rest.to_vec(),
                });
            }
            // If first segment doesn't match and isn't `*`, this pattern
            // doesn't apply at this key -- skip it.
        }

        if remove {
            continue;
        }

        // Recurse into the child with the narrowed pattern set.
        if child_patterns.is_empty() {
            result.insert(key.clone(), child.clone());
        } else {
            result.insert(key.clone(), apply_filter(child, &child_patterns));
        }
    }

    Value::Object(result)
}

/// Filter array elements by passing all patterns through to each element.
///
/// Arrays are "transparent" to pattern matching -- they don't consume
/// any pattern segments. This means `"items.etag"` works correctly when
/// `items` is an array: the pattern descends into each array element.
fn filter_array(arr: &[Value], patterns: &[Pattern<'_>]) -> Value {
    Value::Array(
        arr.iter()
            .map(|elem| apply_filter(elem, patterns))
            .collect(),
    )
}

/// Filter JSON fields by pattern, then encode the result to TOON.
///
/// This is a convenience function combining [`filter_fields`] with
/// [`crate::encode`]. The JSON string is parsed, filtered, re-serialized
/// to JSON, and then encoded to TOON.
///
/// # Errors
///
/// Returns an error if the input is not valid JSON or if TOON encoding fails.
///
/// # Examples
///
/// ```
/// use toon_core::filter_and_encode;
///
/// let json = r#"{"name":"Alice","etag":"abc"}"#;
/// let toon = filter_and_encode(json, &["etag"]).unwrap();
/// assert_eq!(toon, "name: Alice");
/// ```
pub fn filter_and_encode(json: &str, patterns: &[&str]) -> Result<String> {
    let value: Value = serde_json::from_str(json)?;
    let filtered = filter_fields(&value, patterns);
    let filtered_json = serde_json::to_string(&filtered)?;
    crate::encoder::encode(&filtered_json)
}

/// Predefined filter sets for common calendar APIs.
pub struct CalendarFilter;

impl CalendarFilter {
    /// Default filter for Google Calendar API responses.
    ///
    /// Strips the following noise fields that inflate token counts without
    /// carrying scheduling-relevant information:
    ///
    /// - `etag` -- entity tag for HTTP caching
    /// - `kind` -- API resource type identifier
    /// - `htmlLink` -- browser URL (reconstructable from event ID)
    /// - `iCalUID` -- iCalendar UID (redundant with event ID)
    /// - `sequence` -- iCalendar sequence number
    /// - `reminders.useDefault` -- boolean flag for default reminders
    /// - `creator.self` -- boolean indicating self-created
    /// - `organizer.self` -- boolean indicating self-organized
    pub fn google_default() -> Vec<&'static str> {
        vec![
            "etag",
            "kind",
            "htmlLink",
            "iCalUID",
            "sequence",
            "reminders.useDefault",
            "creator.self",
            "organizer.self",
            // Wildcard variants to strip these fields at any nesting depth
            // (e.g., inside items[] in a calendar list response).
            "*.etag",
            "*.kind",
            "*.htmlLink",
            "*.iCalUID",
            "*.sequence",
        ]
    }
}
