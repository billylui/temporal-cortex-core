/// TDD RED phase: Encoder contract tests for TOON v3.0
///
/// These tests define the expected encoding behavior BEFORE the encoder
/// is implemented. All tests should FAIL initially (encoder returns todo!()).
///
/// Spec reference: TOON v3.0 (2025-11-24) — github.com/toon-format/spec
use toon_core::encode;

// ============================================================================
// Primitives
// ============================================================================

#[test]
fn encode_null() {
    let json = r#"null"#;
    let toon = encode(json).unwrap();
    assert_eq!(toon, "null");
}

#[test]
fn encode_bool_true() {
    let json = r#"true"#;
    let toon = encode(json).unwrap();
    assert_eq!(toon, "true");
}

#[test]
fn encode_bool_false() {
    let json = r#"false"#;
    let toon = encode(json).unwrap();
    assert_eq!(toon, "false");
}

#[test]
fn encode_integer() {
    let json = r#"42"#;
    let toon = encode(json).unwrap();
    assert_eq!(toon, "42");
}

#[test]
fn encode_negative_integer() {
    let json = r#"-7"#;
    let toon = encode(json).unwrap();
    assert_eq!(toon, "-7");
}

#[test]
fn encode_float() {
    let json = r#"3.14"#;
    let toon = encode(json).unwrap();
    assert_eq!(toon, "3.14");
}

#[test]
fn encode_float_no_trailing_zeros() {
    // Spec: No trailing fractional zeros. 1.50 -> 1.5
    let json = r#"1.50"#;
    let toon = encode(json).unwrap();
    assert_eq!(toon, "1.5");
}

#[test]
fn encode_float_integer_form() {
    // Spec: Integer form when fractional is zero. 1.0 -> 1
    let json = r#"1.0"#;
    let toon = encode(json).unwrap();
    assert_eq!(toon, "1");
}

#[test]
fn encode_negative_zero() {
    // Spec: -0 MUST be normalized to 0
    let json = r#"-0"#;
    let toon = encode(json).unwrap();
    assert_eq!(toon, "0");
}

#[test]
fn encode_large_number_no_exponent() {
    // Spec: No exponent notation. 1e6 -> 1000000
    // JSON allows 1e6 but serde_json parses it to 1000000.0
    let json = r#"1000000"#;
    let toon = encode(json).unwrap();
    assert_eq!(toon, "1000000");
}

#[test]
fn encode_string_simple() {
    let json = r#""hello world""#;
    let toon = encode(json).unwrap();
    // No colons, no special chars, no leading/trailing whitespace -> unquoted
    assert_eq!(toon, "hello world");
}

#[test]
fn encode_empty_string() {
    // Spec: Empty string MUST be quoted
    let json = r#""""#;
    let toon = encode(json).unwrap();
    assert_eq!(toon, r#""""#);
}

#[test]
fn encode_string_that_looks_like_true() {
    // Spec: String "true" MUST be quoted
    let json = r#""true""#;
    let toon = encode(json).unwrap();
    assert_eq!(toon, r#""true""#);
}

#[test]
fn encode_string_that_looks_like_false() {
    let json = r#""false""#;
    let toon = encode(json).unwrap();
    assert_eq!(toon, r#""false""#);
}

#[test]
fn encode_string_that_looks_like_null() {
    let json = r#""null""#;
    let toon = encode(json).unwrap();
    assert_eq!(toon, r#""null""#);
}

#[test]
fn encode_string_that_looks_like_number() {
    // Spec: Numeric-like strings MUST be quoted
    let json = r#""42""#;
    let toon = encode(json).unwrap();
    assert_eq!(toon, r#""42""#);
}

#[test]
fn encode_string_with_leading_zero() {
    // Spec: "05" is numeric-like, must be quoted
    let json = r#""05""#;
    let toon = encode(json).unwrap();
    assert_eq!(toon, r#""05""#);
}

#[test]
fn encode_string_containing_colon() {
    // Spec: String containing colon MUST be quoted
    let json = r#""hello:world""#;
    let toon = encode(json).unwrap();
    assert_eq!(toon, r#""hello:world""#);
}

#[test]
fn encode_string_containing_backslash() {
    // Spec: String containing backslash MUST be quoted and escaped
    let json = r#""path\\to""#;
    let toon = encode(json).unwrap();
    assert_eq!(toon, r#""path\\to""#);
}

#[test]
fn encode_string_containing_newline() {
    // Spec: Control characters -> must be quoted and escaped
    let json = "\"line1\\nline2\"";
    let toon = encode(json).unwrap();
    assert_eq!(toon, r#""line1\nline2""#);
}

#[test]
fn encode_string_containing_quote() {
    let json = r#""say \"hi\"""#;
    let toon = encode(json).unwrap();
    assert_eq!(toon, r#""say \"hi\"""#);
}

#[test]
fn encode_string_with_leading_whitespace() {
    // Spec: Leading/trailing whitespace requires quoting
    let json = r#""  spaces  ""#;
    let toon = encode(json).unwrap();
    assert_eq!(toon, r#""  spaces  ""#);
}

#[test]
fn encode_string_hyphen() {
    // Spec: String "-" must be quoted
    let json = r#""-""#;
    let toon = encode(json).unwrap();
    assert_eq!(toon, r#""-""#);
}

#[test]
fn encode_string_starts_with_hyphen() {
    // Spec: String starting with "-" must be quoted (could be confused with list item)
    let json = r#""-hello""#;
    let toon = encode(json).unwrap();
    assert_eq!(toon, r#""-hello""#);
}

#[test]
fn encode_string_containing_bracket() {
    // Spec: Brackets/braces require quoting
    let json = r#""[data]""#;
    let toon = encode(json).unwrap();
    assert_eq!(toon, r#""[data]""#);
}

#[test]
fn encode_string_unicode_safe() {
    // Unicode without special chars is safe unquoted
    let json = r#""café""#;
    let toon = encode(json).unwrap();
    assert_eq!(toon, "café");
}

// ============================================================================
// Flat Objects
// ============================================================================

#[test]
fn encode_flat_object() {
    let json = r#"{"id":123,"name":"Ada Lovelace","active":true}"#;
    let toon = encode(json).unwrap();
    let expected = "id: 123\nname: Ada Lovelace\nactive: true";
    assert_eq!(toon, expected);
}

#[test]
fn encode_flat_object_with_null() {
    let json = r#"{"name":"Bob","score":null}"#;
    let toon = encode(json).unwrap();
    assert_eq!(toon, "name: Bob\nscore: null");
}

#[test]
fn encode_flat_object_preserves_key_order() {
    let json = r#"{"z":1,"a":2,"m":3}"#;
    let toon = encode(json).unwrap();
    assert_eq!(toon, "z: 1\na: 2\nm: 3");
}

#[test]
fn encode_empty_object() {
    // Spec: Empty root object = empty document
    let json = r#"{}"#;
    let toon = encode(json).unwrap();
    assert_eq!(toon, "");
}

#[test]
fn encode_object_with_special_string_values() {
    let json = r#"{"keyword":"true","empty":"","url":"http://a:b"}"#;
    let toon = encode(json).unwrap();
    let expected = "keyword: \"true\"\nempty: \"\"\nurl: \"http://a:b\"";
    assert_eq!(toon, expected);
}

#[test]
fn encode_object_key_requiring_quoting() {
    // Spec: Keys with hyphens must be quoted
    let json = r#"{"my-key":"value"}"#;
    let toon = encode(json).unwrap();
    assert_eq!(toon, "\"my-key\": value");
}

// ============================================================================
// Nested Objects
// ============================================================================

#[test]
fn encode_nested_object() {
    let json = r#"{"user":{"id":1,"name":"Ada"}}"#;
    let toon = encode(json).unwrap();
    let expected = "user:\n  id: 1\n  name: Ada";
    assert_eq!(toon, expected);
}

#[test]
fn encode_deeply_nested_object() {
    let json = r#"{"a":{"b":{"c":"deep"}}}"#;
    let toon = encode(json).unwrap();
    let expected = "a:\n  b:\n    c: deep";
    assert_eq!(toon, expected);
}

#[test]
fn encode_mixed_nested_flat() {
    let json = r#"{"name":"App","server":{"host":"localhost","port":8080},"debug":true}"#;
    let toon = encode(json).unwrap();
    let expected = "name: App\nserver:\n  host: localhost\n  port: 8080\ndebug: true";
    assert_eq!(toon, expected);
}

#[test]
fn encode_nested_empty_object() {
    // Spec: key: (colon alone) for empty nested object
    let json = r#"{"config":{}}"#;
    let toon = encode(json).unwrap();
    assert_eq!(toon, "config:");
}

// ============================================================================
// Primitive Arrays (Inline)
// ============================================================================

#[test]
fn encode_primitive_array_integers() {
    let json = r#"{"numbers":[1,2,3,4,5]}"#;
    let toon = encode(json).unwrap();
    assert_eq!(toon, "numbers[5]: 1,2,3,4,5");
}

#[test]
fn encode_primitive_array_strings() {
    let json = r#"{"tags":["admin","ops","dev"]}"#;
    let toon = encode(json).unwrap();
    assert_eq!(toon, "tags[3]: admin,ops,dev");
}

#[test]
fn encode_empty_array() {
    let json = r#"{"items":[]}"#;
    let toon = encode(json).unwrap();
    assert_eq!(toon, "items[0]:");
}

#[test]
fn encode_primitive_array_mixed_types() {
    let json = r#"{"mixed":[1,"hello",true,null]}"#;
    let toon = encode(json).unwrap();
    assert_eq!(toon, "mixed[4]: 1,hello,true,null");
}

#[test]
fn encode_primitive_array_string_needing_quotes() {
    // String containing comma (the default delimiter) must be quoted
    let json = r#"{"items":["a,b","c"]}"#;
    let toon = encode(json).unwrap();
    assert_eq!(toon, r#"items[2]: "a,b",c"#);
}

#[test]
fn encode_root_array() {
    // Spec: Root arrays use headerless syntax: [N]: v1,v2,...
    let json = r#"[1,2,3]"#;
    let toon = encode(json).unwrap();
    assert_eq!(toon, "[3]: 1,2,3");
}

// ============================================================================
// Tabular Arrays (Uniform Objects)
// ============================================================================

#[test]
fn encode_tabular_array_basic() {
    let json =
        r#"{"users":[{"id":1,"name":"Alice","active":true},{"id":2,"name":"Bob","active":false}]}"#;
    let toon = encode(json).unwrap();
    let expected = "users[2]{id,name,active}:\n  1,Alice,true\n  2,Bob,false";
    assert_eq!(toon, expected);
}

#[test]
fn encode_tabular_array_preserves_field_order() {
    // Field order follows first object's key encounter order
    // Values are strings that look like numbers, so they must be quoted to preserve type
    let json = r#"{"items":[{"z":"1","a":"2"},{"z":"3","a":"4"}]}"#;
    let toon = encode(json).unwrap();
    let expected = "items[2]{z,a}:\n  \"1\",\"2\"\n  \"3\",\"4\"";
    assert_eq!(toon, expected);
}

#[test]
fn encode_tabular_with_quoting() {
    // Values containing comma must be quoted in tabular rows
    let json = r#"{"items":[{"name":"a,b","id":1},{"name":"c","id":2}]}"#;
    let toon = encode(json).unwrap();
    let expected = "items[2]{name,id}:\n  \"a,b\",1\n  c,2";
    assert_eq!(toon, expected);
}

#[test]
fn encode_tabular_single_row() {
    let json = r#"{"items":[{"x":1,"y":2}]}"#;
    let toon = encode(json).unwrap();
    let expected = "items[1]{x,y}:\n  1,2";
    assert_eq!(toon, expected);
}

// ============================================================================
// Mixed / Non-Uniform Arrays (Expanded List)
// ============================================================================

#[test]
fn encode_mixed_array() {
    let json = r#"{"items":[1,{"a":"hello","b":"world"},"text"]}"#;
    let toon = encode(json).unwrap();
    let expected = "items[3]:\n  - 1\n  - a: hello\n    b: world\n  - text";
    assert_eq!(toon, expected);
}

#[test]
fn encode_array_of_non_uniform_objects() {
    // Objects with different keys -> not tabular -> list form
    let json = r#"{"items":[{"a":1},{"b":2}]}"#;
    let toon = encode(json).unwrap();
    let expected = "items[2]:\n  - a: 1\n  - b: 2";
    assert_eq!(toon, expected);
}

#[test]
fn encode_root_mixed_array() {
    let json = r#"[1,"hello",true]"#;
    let toon = encode(json).unwrap();
    assert_eq!(toon, "[3]: 1,hello,true");
}

#[test]
fn encode_array_of_arrays() {
    // Nested arrays in list form
    let json = r#"{"matrix":[[1,2],[3,4]]}"#;
    let toon = encode(json).unwrap();
    let expected = "matrix[2]:\n  - [2]: 1,2\n  - [2]: 3,4";
    assert_eq!(toon, expected);
}

// ============================================================================
// Calendar-specific: realistic Google Calendar event payload
// ============================================================================

#[test]
fn encode_calendar_events_tabular() {
    let json = r#"{"summary":"Engineering Sync","timeZone":"America/Los_Angeles","items":[{"id":"evt_1a2b","status":"confirmed","summary":"Q1 Strategy Sync","start":"2026-02-17T10:00:00-08:00","end":"2026-02-17T11:00:00-08:00"},{"id":"evt_9f8e","status":"confirmed","summary":"Vendor Negotiation","start":"2026-02-18T13:00:00-08:00","end":"2026-02-18T14:00:00-08:00"}]}"#;
    let toon = encode(json).unwrap();
    let expected = "\
summary: Engineering Sync
timeZone: America/Los_Angeles
items[2]{id,status,summary,start,end}:
  evt_1a2b,confirmed,Q1 Strategy Sync,2026-02-17T10:00:00-08:00,2026-02-17T11:00:00-08:00
  evt_9f8e,confirmed,Vendor Negotiation,2026-02-18T13:00:00-08:00,2026-02-18T14:00:00-08:00";
    assert_eq!(toon, expected);
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn encode_string_with_tab() {
    // Tab in string value -> must be quoted and escaped
    let json = "\"col1\\tcol2\"";
    let toon = encode(json).unwrap();
    assert_eq!(toon, "\"col1\\tcol2\"");
}

#[test]
fn encode_object_with_numeric_string_key() {
    // Key "123" requires quoting (starts with digit)
    let json = r#"{"123":"value"}"#;
    let toon = encode(json).unwrap();
    assert_eq!(toon, "\"123\": value");
}

#[test]
fn encode_single_key_chain_no_folding() {
    // With key folding OFF (default), nested single-key chains stay nested
    let json = r#"{"server":{"host":"localhost"}}"#;
    let toon = encode(json).unwrap();
    let expected = "server:\n  host: localhost";
    assert_eq!(toon, expected);
}

#[test]
fn encode_objects_with_nested_values_not_tabular() {
    // Objects with nested object values -> not tabular (values must be primitive)
    let json = r#"{"items":[{"a":{"x":1}},{"a":{"x":2}}]}"#;
    let toon = encode(json).unwrap();
    let expected = "items[2]:\n  - a:\n      x: 1\n  - a:\n      x: 2";
    assert_eq!(toon, expected);
}

#[test]
fn encode_no_trailing_newline() {
    // Spec: No trailing newline at end of document
    let json = r#"{"a":1}"#;
    let toon = encode(json).unwrap();
    assert!(
        !toon.ends_with('\n'),
        "TOON output must not end with newline"
    );
}

#[test]
fn encode_no_trailing_spaces() {
    // Spec: No trailing spaces at end of any line
    let json = r#"{"a":1,"b":"hello"}"#;
    let toon = encode(json).unwrap();
    for (i, line) in toon.lines().enumerate() {
        assert!(
            !line.ends_with(' '),
            "Line {} has trailing space: {:?}",
            i,
            line
        );
    }
}

#[test]
fn encode_timestamp_value_quoted() {
    // Timestamp with colons must be quoted as a value
    let json = r#"{"timestamp":"2025-01-15T10:30:00Z"}"#;
    let toon = encode(json).unwrap();
    assert_eq!(toon, "timestamp: \"2025-01-15T10:30:00Z\"");
}

// Note on datetime in tabular: inside tabular rows, the quoting depends on
// the active delimiter. With comma delimiter, colons in values don't need
// quoting per spec (quoting triggered by active delimiter, not colon).
// But as a top-level object value, colon triggers quoting.
#[test]
fn encode_tabular_datetime_no_extra_quotes() {
    // In tabular rows with comma delimiter, colons DON'T trigger quoting
    // (only the active delimiter does for row cells)
    let json = r#"{"events":[{"time":"10:30:00","name":"meeting"}]}"#;
    let toon = encode(json).unwrap();
    // In tabular rows, colon doesn't require quoting (comma is active delimiter)
    let expected = "events[1]{time,name}:\n  10:30:00,meeting";
    assert_eq!(toon, expected);
}
