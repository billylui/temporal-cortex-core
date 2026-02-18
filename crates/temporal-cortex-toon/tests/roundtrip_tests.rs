use toon_core::{decode, encode};

/// Assert that encode → decode roundtrips to the same JSON value.
fn assert_roundtrip(json: &str) {
    let toon = encode(json).expect("encode failed");
    let decoded = decode(&toon).expect("decode failed");
    let original: serde_json::Value = serde_json::from_str(json).unwrap();
    let roundtripped: serde_json::Value = serde_json::from_str(&decoded).unwrap();
    assert_eq!(
        original, roundtripped,
        "Roundtrip failed:\n  input JSON: {json}\n  TOON:       {toon}\n  output JSON: {decoded}"
    );
}

// ============================================================================
// Primitive Roundtrips
// ============================================================================

#[test]
fn roundtrip_null() {
    assert_roundtrip("null");
}

#[test]
fn roundtrip_bool_true() {
    assert_roundtrip("true");
}

#[test]
fn roundtrip_bool_false() {
    assert_roundtrip("false");
}

#[test]
fn roundtrip_integer() {
    assert_roundtrip("42");
}

#[test]
fn roundtrip_negative_integer() {
    assert_roundtrip("-7");
}

#[test]
fn roundtrip_float() {
    assert_roundtrip("3.14");
}

#[test]
fn roundtrip_zero() {
    assert_roundtrip("0");
}

#[test]
fn roundtrip_string() {
    assert_roundtrip(r#""hello""#);
}

#[test]
fn roundtrip_empty_string() {
    assert_roundtrip(r#""""#);
}

#[test]
fn roundtrip_string_with_newline() {
    assert_roundtrip(r#""line1\nline2""#);
}

#[test]
fn roundtrip_string_with_backslash() {
    assert_roundtrip(r#""path\\to\\file""#);
}

#[test]
fn roundtrip_string_with_quote() {
    assert_roundtrip(r#""say \"hi\"""#);
}

#[test]
fn roundtrip_string_with_tab() {
    assert_roundtrip(r#""col1\tcol2""#);
}

// ============================================================================
// Object Roundtrips
// ============================================================================

#[test]
fn roundtrip_flat_object() {
    assert_roundtrip(r#"{"name":"Alice","age":30,"active":true}"#);
}

#[test]
fn roundtrip_object_with_null() {
    assert_roundtrip(r#"{"name":"Alice","email":null}"#);
}

#[test]
fn roundtrip_nested_object() {
    assert_roundtrip(r#"{"server":{"host":"localhost","port":8080}}"#);
}

#[test]
fn roundtrip_deeply_nested() {
    assert_roundtrip(r#"{"a":{"b":{"c":"deep"}}}"#);
}

#[test]
fn roundtrip_mixed_nested_flat() {
    assert_roundtrip(r#"{"name":"App","server":{"host":"localhost","port":8080},"debug":true}"#);
}

#[test]
fn roundtrip_empty_object() {
    assert_roundtrip("{}");
}

#[test]
fn roundtrip_nested_empty_object() {
    assert_roundtrip(r#"{"meta":{}}"#);
}

#[test]
fn roundtrip_quoted_key() {
    assert_roundtrip(r#"{"my key":"value"}"#);
}

#[test]
fn roundtrip_object_with_special_strings() {
    assert_roundtrip(r#"{"a":"","b":"true","c":"null","d":"42","e":"05","f":"hello:world"}"#);
}

// ============================================================================
// Array Roundtrips
// ============================================================================

#[test]
fn roundtrip_inline_array() {
    assert_roundtrip(r#"{"ids":[1,2,3]}"#);
}

#[test]
fn roundtrip_string_array() {
    assert_roundtrip(r#"{"tags":["red","blue","green"]}"#);
}

#[test]
fn roundtrip_mixed_type_array() {
    assert_roundtrip(r#"{"data":["hello",42,true,null]}"#);
}

#[test]
fn roundtrip_empty_array() {
    assert_roundtrip(r#"{"items":[]}"#);
}

#[test]
fn roundtrip_root_array() {
    assert_roundtrip("[1,2,3]");
}

// ============================================================================
// Tabular Roundtrips
// ============================================================================

#[test]
fn roundtrip_tabular_array() {
    assert_roundtrip(
        r#"{"users":[{"id":1,"name":"Alice","active":true},{"id":2,"name":"Bob","active":false}]}"#,
    );
}

#[test]
fn roundtrip_tabular_with_quoted_comma() {
    assert_roundtrip(r#"{"items":[{"name":"a,b","id":1},{"name":"c","id":2}]}"#);
}

#[test]
fn roundtrip_tabular_single_row() {
    assert_roundtrip(r#"{"data":[{"x":10,"y":20}]}"#);
}

// ============================================================================
// Mixed Array Roundtrips
// ============================================================================

#[test]
fn roundtrip_heterogeneous_array() {
    assert_roundtrip(r#"{"items":["hello",{"name":"test"},[1,2]]}"#);
}

#[test]
fn roundtrip_array_of_objects() {
    assert_roundtrip(r#"{"items":[{"name":"Alice","age":30},{"name":"Bob","age":25}]}"#);
}

#[test]
fn roundtrip_non_uniform_objects() {
    assert_roundtrip(r#"{"items":[{"a":1},{"b":2}]}"#);
}

#[test]
fn roundtrip_array_of_arrays() {
    assert_roundtrip(r#"{"matrix":[[1,2,3],[4,5,6]]}"#);
}

#[test]
fn roundtrip_root_mixed_array() {
    assert_roundtrip(r#"["hello",[1,2],{"name":"Alice","age":30}]"#);
}

// ============================================================================
// Complex / Calendar-like Roundtrips
// ============================================================================

#[test]
fn roundtrip_calendar_event() {
    assert_roundtrip(
        r#"{"summary":"Team Standup","start":"2024-01-15T10:00:00Z","end":"2024-01-15T10:30:00Z","attendees":[{"email":"alice@co.com","name":"Alice","status":"accepted"},{"email":"bob@co.com","name":"Bob","status":"tentative"}]}"#,
    );
}

#[test]
fn roundtrip_object_with_nested_array_and_object() {
    assert_roundtrip(
        r#"{"name":"project","config":{"debug":true,"port":3000},"tags":["web","api"]}"#,
    );
}

#[test]
fn roundtrip_list_item_with_nested_object() {
    assert_roundtrip(
        r#"{"people":[{"name":"Alice","address":{"city":"Portland","zip":"97201"}}]}"#,
    );
}

#[test]
fn roundtrip_list_item_with_array_field() {
    assert_roundtrip(r#"{"items":[{"name":"Alice","tags":["admin","user"]}]}"#);
}

// ============================================================================
// Number Edge Cases
// ============================================================================

#[test]
fn roundtrip_negative_zero() {
    // -0 normalizes to 0
    let toon = encode("-0").unwrap();
    let json = decode(&toon).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v, serde_json::Value::Number(0.into()));
}

#[test]
fn roundtrip_large_integer() {
    assert_roundtrip("999999999");
}

#[test]
fn roundtrip_exponent_to_plain() {
    // 1e2 → 100 in TOON (no exponents), decoded back to 100
    let toon = encode("1e2").unwrap();
    assert_eq!(toon, "100");
    let json = decode(&toon).unwrap();
    assert_json_eq(&json, "100");
}

fn assert_json_eq(a: &str, b: &str) {
    let va: serde_json::Value = serde_json::from_str(a).unwrap();
    let vb: serde_json::Value = serde_json::from_str(b).unwrap();
    assert_eq!(va, vb, "JSON mismatch:\n  actual: {a}\n  expected: {b}");
}
