/// TDD RED phase: Semantic filtering tests for temporal-cortex-toon.
///
/// These tests define the expected behavior of the filter module BEFORE
/// implementation. All tests should FAIL initially (functions return todo!()).
///
/// The filter module strips unnecessary fields from JSON before TOON encoding,
/// reducing token consumption for LLM processing of calendar data.
use toon_core::{encode, filter_and_encode, filter_fields, CalendarFilter};

// ============================================================================
// Helper: Realistic Google Calendar-like JSON fixtures
// ============================================================================

/// Minimal single-event Google Calendar JSON.
fn single_event_json() -> &'static str {
    r#"{"etag":"\"abc123\"","kind":"calendar#event","summary":"Team Standup","start":{"dateTime":"2025-06-15T09:00:00-07:00","timeZone":"America/Los_Angeles"},"end":{"dateTime":"2025-06-15T09:30:00-07:00","timeZone":"America/Los_Angeles"},"htmlLink":"https://calendar.google.com/event?eid=abc123","iCalUID":"abc123@google.com","sequence":0,"status":"confirmed","creator":{"email":"alice@example.com","self":true},"organizer":{"email":"alice@example.com","self":true},"reminders":{"useDefault":true}}"#
}

/// Multi-event Google Calendar API response with nested arrays.
fn calendar_list_json() -> &'static str {
    r#"{"kind":"calendar#events","etag":"\"list-etag\"","summary":"Alice's Calendar","items":[{"etag":"\"ev1-etag\"","kind":"calendar#event","summary":"Team Standup","htmlLink":"https://calendar.google.com/event?eid=ev1","start":{"dateTime":"2025-06-15T09:00:00-07:00"},"end":{"dateTime":"2025-06-15T09:30:00-07:00"},"status":"confirmed","iCalUID":"ev1@google.com","sequence":0,"attendees":[{"email":"alice@example.com","responseStatus":"accepted","self":true},{"email":"bob@example.com","responseStatus":"needsAction"}],"reminders":{"useDefault":true},"creator":{"email":"alice@example.com","self":true},"organizer":{"email":"alice@example.com","self":true}},{"etag":"\"ev2-etag\"","kind":"calendar#event","summary":"Lunch with Bob","htmlLink":"https://calendar.google.com/event?eid=ev2","start":{"dateTime":"2025-06-15T12:00:00-07:00"},"end":{"dateTime":"2025-06-15T13:00:00-07:00"},"status":"confirmed","iCalUID":"ev2@google.com","sequence":1,"attendees":[{"email":"alice@example.com","responseStatus":"accepted"},{"email":"bob@example.com","responseStatus":"accepted","organizer":true}],"reminders":{"useDefault":false,"overrides":[{"method":"popup","minutes":10}]},"creator":{"email":"bob@example.com"},"organizer":{"email":"bob@example.com"}}]}"#
}

/// Deeply nested JSON for testing multi-level filtering.
fn deep_nested_json() -> &'static str {
    r#"{"level1":{"etag":"l1","level2":{"etag":"l2","level3":{"etag":"l3","value":"keep-me"},"data":"also-keep"}}}"#
}

/// Simple flat JSON for basic tests.
fn flat_json() -> &'static str {
    r#"{"name":"Alice","etag":"\"tag1\"","kind":"calendar#event","age":30}"#
}

// ============================================================================
// 1. Basic field stripping
// ============================================================================

#[test]
fn filter_strips_top_level_fields() {
    let result = filter_and_encode(flat_json(), &["etag", "kind"]).unwrap();
    let decoded: serde_json::Value =
        serde_json::from_str(&toon_core::decode(&result).unwrap()).unwrap();

    assert!(decoded.get("name").is_some(), "name should be preserved");
    assert!(decoded.get("age").is_some(), "age should be preserved");
    assert!(decoded.get("etag").is_none(), "etag should be stripped");
    assert!(decoded.get("kind").is_none(), "kind should be stripped");
}

#[test]
fn filter_strips_single_top_level_field() {
    let result = filter_and_encode(flat_json(), &["etag"]).unwrap();
    let decoded: serde_json::Value =
        serde_json::from_str(&toon_core::decode(&result).unwrap()).unwrap();

    assert!(decoded.get("etag").is_none(), "etag should be stripped");
    assert!(decoded.get("name").is_some(), "name should be preserved");
    assert!(decoded.get("kind").is_some(), "kind should be preserved");
    assert!(decoded.get("age").is_some(), "age should be preserved");
}

#[test]
fn filter_fields_returns_value_without_stripped_keys() {
    let value: serde_json::Value = serde_json::from_str(flat_json()).unwrap();
    let filtered = filter_fields(&value, &["etag", "kind"]);

    assert!(filtered.get("name").is_some());
    assert!(filtered.get("age").is_some());
    assert!(filtered.get("etag").is_none());
    assert!(filtered.get("kind").is_none());
}

// ============================================================================
// 2. Nested field stripping
// ============================================================================

#[test]
fn filter_strips_nested_fields_with_dot_path() {
    let result =
        filter_and_encode(calendar_list_json(), &["items.etag", "items.htmlLink"]).unwrap();
    let decoded: serde_json::Value =
        serde_json::from_str(&toon_core::decode(&result).unwrap()).unwrap();

    // Top-level etag should still be present (we only stripped items.etag)
    assert!(
        decoded.get("etag").is_some(),
        "top-level etag should be preserved"
    );

    // Items should have etag and htmlLink stripped
    let items = decoded.get("items").unwrap().as_array().unwrap();
    for (i, item) in items.iter().enumerate() {
        assert!(
            item.get("etag").is_none(),
            "items[{i}].etag should be stripped"
        );
        assert!(
            item.get("htmlLink").is_none(),
            "items[{i}].htmlLink should be stripped"
        );
        // Other fields preserved
        assert!(
            item.get("summary").is_some(),
            "items[{i}].summary should be preserved"
        );
    }
}

#[test]
fn filter_strips_doubly_nested_field() {
    let json =
        r#"{"event":{"creator":{"email":"alice@example.com","self":true},"summary":"Test"}}"#;
    let result = filter_and_encode(json, &["event.creator.self"]).unwrap();
    let decoded: serde_json::Value =
        serde_json::from_str(&toon_core::decode(&result).unwrap()).unwrap();

    let creator = decoded.get("event").unwrap().get("creator").unwrap();
    assert!(
        creator.get("self").is_none(),
        "creator.self should be stripped"
    );
    assert!(
        creator.get("email").is_some(),
        "creator.email should be preserved"
    );
}

// ============================================================================
// 3. Wildcard patterns
// ============================================================================

#[test]
fn filter_wildcard_strips_field_at_any_depth() {
    let result = filter_and_encode(deep_nested_json(), &["*.etag"]).unwrap();
    let decoded: serde_json::Value =
        serde_json::from_str(&toon_core::decode(&result).unwrap()).unwrap();

    // All etag fields at every level should be gone
    assert!(
        decoded.get("level1").unwrap().get("etag").is_none(),
        "level1.etag should be stripped"
    );
    assert!(
        decoded
            .get("level1")
            .unwrap()
            .get("level2")
            .unwrap()
            .get("etag")
            .is_none(),
        "level2.etag should be stripped"
    );
    assert!(
        decoded
            .get("level1")
            .unwrap()
            .get("level2")
            .unwrap()
            .get("level3")
            .unwrap()
            .get("etag")
            .is_none(),
        "level3.etag should be stripped"
    );

    // Non-etag fields preserved
    assert_eq!(decoded["level1"]["level2"]["level3"]["value"], "keep-me");
    assert_eq!(decoded["level1"]["level2"]["data"], "also-keep");
}

#[test]
fn filter_wildcard_strips_etag_from_calendar_at_all_levels() {
    let result = filter_and_encode(calendar_list_json(), &["*.etag"]).unwrap();
    let decoded: serde_json::Value =
        serde_json::from_str(&toon_core::decode(&result).unwrap()).unwrap();

    // Top-level etag stripped
    assert!(
        decoded.get("etag").is_none(),
        "top-level etag should be stripped"
    );

    // Item-level etags stripped
    let items = decoded.get("items").unwrap().as_array().unwrap();
    for (i, item) in items.iter().enumerate() {
        assert!(
            item.get("etag").is_none(),
            "items[{i}].etag should be stripped by wildcard"
        );
    }
}

// ============================================================================
// 4. Array element filtering
// ============================================================================

#[test]
fn filter_strips_fields_inside_array_elements() {
    let result =
        filter_and_encode(calendar_list_json(), &["items.attendees.*.responseStatus"]).unwrap();
    let decoded: serde_json::Value =
        serde_json::from_str(&toon_core::decode(&result).unwrap()).unwrap();

    let items = decoded.get("items").unwrap().as_array().unwrap();
    for (i, item) in items.iter().enumerate() {
        if let Some(attendees) = item.get("attendees") {
            for (j, attendee) in attendees.as_array().unwrap().iter().enumerate() {
                assert!(
                    attendee.get("responseStatus").is_none(),
                    "items[{i}].attendees[{j}].responseStatus should be stripped"
                );
                assert!(
                    attendee.get("email").is_some(),
                    "items[{i}].attendees[{j}].email should be preserved"
                );
            }
        }
    }
}

#[test]
fn filter_strips_self_flag_inside_array_elements_via_wildcard() {
    let result = filter_and_encode(calendar_list_json(), &["items.attendees.*.self"]).unwrap();
    let decoded: serde_json::Value =
        serde_json::from_str(&toon_core::decode(&result).unwrap()).unwrap();

    let items = decoded.get("items").unwrap().as_array().unwrap();
    // First event has attendee with self:true
    let attendees = items[0].get("attendees").unwrap().as_array().unwrap();
    for attendee in attendees {
        assert!(
            attendee.get("self").is_none(),
            "attendee.self should be stripped"
        );
    }
}

// ============================================================================
// 5. No matching fields -- unchanged output
// ============================================================================

#[test]
fn filter_with_nonexistent_fields_returns_same_as_unfiltered() {
    let unfiltered = encode(flat_json()).unwrap();
    let filtered = filter_and_encode(flat_json(), &["nonExistentField", "alsoFake"]).unwrap();

    assert_eq!(
        filtered, unfiltered,
        "filtering non-existent fields should produce identical output"
    );
}

#[test]
fn filter_fields_with_nonexistent_path_preserves_all_fields() {
    let value: serde_json::Value = serde_json::from_str(flat_json()).unwrap();
    let filtered = filter_fields(&value, &["does.not.exist"]);

    assert_eq!(
        filtered, value,
        "filtering non-existent path should return identical value"
    );
}

// ============================================================================
// 6. Empty filter list -- no fields stripped
// ============================================================================

#[test]
fn filter_with_empty_patterns_returns_same_as_unfiltered() {
    let unfiltered = encode(flat_json()).unwrap();
    let filtered = filter_and_encode(flat_json(), &[]).unwrap();

    assert_eq!(
        filtered, unfiltered,
        "empty filter list should produce identical output"
    );
}

#[test]
fn filter_fields_with_empty_patterns_preserves_all_fields() {
    let value: serde_json::Value = serde_json::from_str(single_event_json()).unwrap();
    let filtered = filter_fields(&value, &[]);

    assert_eq!(
        filtered, value,
        "empty filter should return identical value"
    );
}

// ============================================================================
// 7. CalendarFilter::google_default() preset
// ============================================================================

#[test]
fn google_default_filter_contains_expected_patterns() {
    let patterns = CalendarFilter::google_default();

    assert!(patterns.contains(&"etag"), "should include etag");
    assert!(patterns.contains(&"kind"), "should include kind");
    assert!(patterns.contains(&"htmlLink"), "should include htmlLink");
    assert!(patterns.contains(&"iCalUID"), "should include iCalUID");
    assert!(patterns.contains(&"sequence"), "should include sequence");
    assert!(
        patterns.contains(&"reminders.useDefault"),
        "should include reminders.useDefault"
    );
    assert!(
        patterns.contains(&"creator.self"),
        "should include creator.self"
    );
    assert!(
        patterns.contains(&"organizer.self"),
        "should include organizer.self"
    );
}

#[test]
fn google_default_filter_strips_noise_from_single_event() {
    let patterns = CalendarFilter::google_default();
    let pattern_refs: Vec<&str> = patterns.to_vec();
    let result = filter_and_encode(single_event_json(), &pattern_refs).unwrap();
    let decoded: serde_json::Value =
        serde_json::from_str(&toon_core::decode(&result).unwrap()).unwrap();

    // Noise fields stripped
    assert!(decoded.get("etag").is_none(), "etag should be stripped");
    assert!(decoded.get("kind").is_none(), "kind should be stripped");
    assert!(
        decoded.get("htmlLink").is_none(),
        "htmlLink should be stripped"
    );
    assert!(
        decoded.get("iCalUID").is_none(),
        "iCalUID should be stripped"
    );
    assert!(
        decoded.get("sequence").is_none(),
        "sequence should be stripped"
    );

    // Nested noise stripped
    let reminders = decoded.get("reminders").unwrap();
    assert!(
        reminders.get("useDefault").is_none(),
        "reminders.useDefault should be stripped"
    );
    let creator = decoded.get("creator").unwrap();
    assert!(
        creator.get("self").is_none(),
        "creator.self should be stripped"
    );
    let organizer = decoded.get("organizer").unwrap();
    assert!(
        organizer.get("self").is_none(),
        "organizer.self should be stripped"
    );

    // Meaningful fields preserved
    assert!(
        decoded.get("summary").is_some(),
        "summary should be preserved"
    );
    assert!(decoded.get("start").is_some(), "start should be preserved");
    assert!(decoded.get("end").is_some(), "end should be preserved");
    assert!(
        decoded.get("status").is_some(),
        "status should be preserved"
    );
    assert!(
        creator.get("email").is_some(),
        "creator.email should be preserved"
    );
}

#[test]
fn google_default_filter_strips_noise_from_event_list() {
    let patterns = CalendarFilter::google_default();
    let pattern_refs: Vec<&str> = patterns.to_vec();
    let result = filter_and_encode(calendar_list_json(), &pattern_refs).unwrap();
    let decoded: serde_json::Value =
        serde_json::from_str(&toon_core::decode(&result).unwrap()).unwrap();

    // Top-level noise stripped
    assert!(decoded.get("etag").is_none());
    assert!(decoded.get("kind").is_none());

    // Item-level noise stripped
    let items = decoded.get("items").unwrap().as_array().unwrap();
    assert_eq!(items.len(), 2, "both events should be preserved");
    for item in items {
        assert!(item.get("etag").is_none());
        assert!(item.get("kind").is_none());
        assert!(item.get("htmlLink").is_none());
        assert!(item.get("iCalUID").is_none());
        assert!(item.get("sequence").is_none());
        assert!(item.get("summary").is_some(), "summary should be preserved");
    }
}

// ============================================================================
// 8. Filtered output is shorter
// ============================================================================

#[test]
fn filtered_toon_is_shorter_than_unfiltered() {
    let unfiltered = encode(single_event_json()).unwrap();
    let patterns = CalendarFilter::google_default();
    let pattern_refs: Vec<&str> = patterns.to_vec();
    let filtered = filter_and_encode(single_event_json(), &pattern_refs).unwrap();

    assert!(
        filtered.len() < unfiltered.len(),
        "filtered output ({} bytes) should be shorter than unfiltered ({} bytes)",
        filtered.len(),
        unfiltered.len()
    );
}

#[test]
fn filtered_calendar_list_is_shorter_than_unfiltered() {
    let unfiltered = encode(calendar_list_json()).unwrap();
    let patterns = CalendarFilter::google_default();
    let pattern_refs: Vec<&str> = patterns.to_vec();
    let filtered = filter_and_encode(calendar_list_json(), &pattern_refs).unwrap();

    assert!(
        filtered.len() < unfiltered.len(),
        "filtered list ({} bytes) should be shorter than unfiltered ({} bytes)",
        filtered.len(),
        unfiltered.len()
    );
}

// ============================================================================
// 9. Roundtrip of filtered data
// ============================================================================

#[test]
fn roundtrip_filtered_single_event_produces_valid_json() {
    let patterns = CalendarFilter::google_default();
    let pattern_refs: Vec<&str> = patterns.to_vec();
    let toon = filter_and_encode(single_event_json(), &pattern_refs).unwrap();
    let json_back = toon_core::decode(&toon).unwrap();

    // Should parse as valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&json_back).unwrap();
    assert!(parsed.is_object(), "decoded result should be a JSON object");

    // Filtered fields should not reappear
    assert!(
        parsed.get("etag").is_none(),
        "etag should not reappear after roundtrip"
    );
    assert!(
        parsed.get("kind").is_none(),
        "kind should not reappear after roundtrip"
    );

    // Meaningful fields survive roundtrip
    assert_eq!(parsed["summary"], "Team Standup");
}

#[test]
fn roundtrip_filtered_calendar_list_produces_valid_json() {
    let patterns = CalendarFilter::google_default();
    let pattern_refs: Vec<&str> = patterns.to_vec();
    let toon = filter_and_encode(calendar_list_json(), &pattern_refs).unwrap();
    let json_back = toon_core::decode(&toon).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&json_back).unwrap();
    let items = parsed.get("items").unwrap().as_array().unwrap();
    assert_eq!(items.len(), 2, "both events should survive roundtrip");
    assert_eq!(items[0]["summary"], "Team Standup");
    assert_eq!(items[1]["summary"], "Lunch with Bob");
}

#[test]
fn roundtrip_flat_filtered_json() {
    let toon = filter_and_encode(flat_json(), &["etag", "kind"]).unwrap();
    let json_back = toon_core::decode(&toon).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_back).unwrap();

    assert_eq!(parsed["name"], "Alice");
    assert_eq!(parsed["age"], 30);
    assert!(parsed.get("etag").is_none());
    assert!(parsed.get("kind").is_none());
}

// ============================================================================
// 10. Deep nesting (3+ levels)
// ============================================================================

#[test]
fn filter_works_on_three_level_nested_path() {
    let json = r#"{"a":{"b":{"c":{"target":"remove-me","keep":"yes"},"other":"preserved"}}}"#;
    let result = filter_and_encode(json, &["a.b.c.target"]).unwrap();
    let decoded: serde_json::Value =
        serde_json::from_str(&toon_core::decode(&result).unwrap()).unwrap();

    assert!(
        decoded["a"]["b"]["c"].get("target").is_none(),
        "a.b.c.target should be stripped"
    );
    assert_eq!(
        decoded["a"]["b"]["c"]["keep"], "yes",
        "a.b.c.keep should be preserved"
    );
    assert_eq!(
        decoded["a"]["b"]["other"], "preserved",
        "a.b.other should be preserved"
    );
}

#[test]
fn filter_wildcard_on_deeply_nested_structures() {
    let json = r#"{"l1":{"noise":"remove","l2":{"noise":"remove","l3":{"noise":"remove","l4":{"noise":"remove","data":"keep"}}}}}"#;
    let result = filter_and_encode(json, &["*.noise"]).unwrap();
    let decoded: serde_json::Value =
        serde_json::from_str(&toon_core::decode(&result).unwrap()).unwrap();

    assert!(
        decoded["l1"].get("noise").is_none(),
        "l1.noise should be stripped"
    );
    assert!(
        decoded["l1"]["l2"].get("noise").is_none(),
        "l2.noise should be stripped"
    );
    assert!(
        decoded["l1"]["l2"]["l3"].get("noise").is_none(),
        "l3.noise should be stripped"
    );
    assert!(
        decoded["l1"]["l2"]["l3"]["l4"].get("noise").is_none(),
        "l4.noise should be stripped"
    );
    assert_eq!(
        decoded["l1"]["l2"]["l3"]["l4"]["data"], "keep",
        "data should be preserved at deepest level"
    );
}

#[test]
fn filter_deep_nested_with_arrays_and_objects() {
    let json = r#"{"events":[{"details":{"location":{"etag":"loc-tag","name":"Conference Room"},"etag":"det-tag"},"etag":"ev-tag"}]}"#;
    let result = filter_and_encode(json, &["*.etag"]).unwrap();
    let decoded: serde_json::Value =
        serde_json::from_str(&toon_core::decode(&result).unwrap()).unwrap();

    let event = &decoded["events"].as_array().unwrap()[0];
    assert!(event.get("etag").is_none(), "event.etag should be stripped");
    assert!(
        event["details"].get("etag").is_none(),
        "details.etag should be stripped"
    );
    assert!(
        event["details"]["location"].get("etag").is_none(),
        "location.etag should be stripped"
    );
    assert_eq!(
        event["details"]["location"]["name"], "Conference Room",
        "location.name should be preserved"
    );
}
