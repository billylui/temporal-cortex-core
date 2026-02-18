use toon_core::decode;

/// Helper: parse JSON strings for comparison, normalizing formatting.
fn json_eq(a: &str, b: &str) -> bool {
    let va: serde_json::Value = serde_json::from_str(a).unwrap();
    let vb: serde_json::Value = serde_json::from_str(b).unwrap();
    va == vb
}

fn assert_json_eq(actual: &str, expected: &str) {
    assert!(
        json_eq(actual, expected),
        "JSON mismatch:\n  actual:   {actual}\n  expected: {expected}"
    );
}

// ============================================================================
// Primitive Values (Root-Level)
// ============================================================================

#[test]
fn decode_null() {
    let json = decode("null").unwrap();
    assert_json_eq(&json, "null");
}

#[test]
fn decode_bool_true() {
    let json = decode("true").unwrap();
    assert_json_eq(&json, "true");
}

#[test]
fn decode_bool_false() {
    let json = decode("false").unwrap();
    assert_json_eq(&json, "false");
}

#[test]
fn decode_integer() {
    let json = decode("42").unwrap();
    assert_json_eq(&json, "42");
}

#[test]
fn decode_negative_integer() {
    let json = decode("-7").unwrap();
    assert_json_eq(&json, "-7");
}

#[test]
fn decode_float() {
    let json = decode("3.14").unwrap();
    assert_json_eq(&json, "3.14");
}

#[test]
fn decode_zero() {
    let json = decode("0").unwrap();
    assert_json_eq(&json, "0");
}

#[test]
fn decode_quoted_string() {
    let json = decode("\"hello world\"").unwrap();
    assert_json_eq(&json, r#""hello world""#);
}

#[test]
fn decode_unquoted_string() {
    // Unquoted string that doesn't look like a keyword or number
    let json = decode("hello").unwrap();
    assert_json_eq(&json, r#""hello""#);
}

#[test]
fn decode_quoted_empty_string() {
    let json = decode("\"\"").unwrap();
    assert_json_eq(&json, r#""""#);
}

#[test]
fn decode_quoted_string_with_escapes() {
    let json = decode(r#""line1\nline2""#).unwrap();
    assert_json_eq(&json, r#""line1\nline2""#);
}

#[test]
fn decode_quoted_string_with_backslash() {
    let json = decode(r#""path\\to\\file""#).unwrap();
    assert_json_eq(&json, r#""path\\to\\file""#);
}

#[test]
fn decode_quoted_string_with_inner_quote() {
    let json = decode(r#""say \"hi\"""#).unwrap();
    assert_json_eq(&json, r#""say \"hi\"""#);
}

// ============================================================================
// Flat Objects
// ============================================================================

#[test]
fn decode_flat_object() {
    let toon = "name: Alice\nage: 30\nactive: true";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, r#"{"name":"Alice","age":30,"active":true}"#);
}

#[test]
fn decode_flat_object_with_null() {
    let toon = "name: Alice\nemail: null";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, r#"{"name":"Alice","email":null}"#);
}

#[test]
fn decode_flat_object_with_quoted_value() {
    let toon = "name: Alice\ntime: \"2024-01-15T10:30:00Z\"";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, r#"{"name":"Alice","time":"2024-01-15T10:30:00Z"}"#);
}

#[test]
fn decode_flat_object_unquoted_string_value() {
    // A value that doesn't look like bool/null/number should decode as string
    let toon = "city: Portland";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, r#"{"city":"Portland"}"#);
}

#[test]
fn decode_empty_object() {
    // An empty TOON document (no lines) should decode to empty object
    let json = decode("").unwrap();
    assert_json_eq(&json, "{}");
}

#[test]
fn decode_object_with_quoted_key() {
    let toon = "\"my key\": value";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, r#"{"my key":"value"}"#);
}

#[test]
fn decode_object_with_numeric_string_value() {
    // Quoted "42" should decode as string, not number
    let toon = "code: \"42\"";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, r#"{"code":"42"}"#);
}

#[test]
fn decode_object_with_bool_string_value() {
    // Quoted "true" should decode as string, not bool
    let toon = "label: \"true\"";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, r#"{"label":"true"}"#);
}

// ============================================================================
// Nested Objects
// ============================================================================

#[test]
fn decode_nested_object() {
    let toon = "server:\n  host: localhost\n  port: 8080";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, r#"{"server":{"host":"localhost","port":8080}}"#);
}

#[test]
fn decode_deeply_nested_object() {
    let toon = "a:\n  b:\n    c: deep";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, r#"{"a":{"b":{"c":"deep"}}}"#);
}

#[test]
fn decode_mixed_nested_flat() {
    let toon = "name: App\nserver:\n  host: localhost\n  port: 8080\ndebug: true";
    let json = decode(toon).unwrap();
    assert_json_eq(
        &json,
        r#"{"name":"App","server":{"host":"localhost","port":8080},"debug":true}"#,
    );
}

#[test]
fn decode_nested_empty_object() {
    // "key:" with no value and no children = empty object
    let toon = "meta:";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, r#"{"meta":{}}"#);
}

#[test]
fn decode_nested_empty_object_with_sibling() {
    let toon = "meta:\nname: test";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, r#"{"meta":{},"name":"test"}"#);
}

// ============================================================================
// Inline Arrays (Primitive)
// ============================================================================

#[test]
fn decode_inline_array_integers() {
    let toon = "ids[3]: 1,2,3";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, r#"{"ids":[1,2,3]}"#);
}

#[test]
fn decode_inline_array_strings() {
    let toon = "tags[2]: red,blue";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, r#"{"tags":["red","blue"]}"#);
}

#[test]
fn decode_inline_array_mixed_types() {
    let toon = "data[4]: hello,42,true,null";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, r#"{"data":["hello",42,true,null]}"#);
}

#[test]
fn decode_inline_array_with_quoted_value() {
    let toon = "items[2]: \"a,b\",c";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, r#"{"items":["a,b","c"]}"#);
}

#[test]
fn decode_empty_array() {
    let toon = "items[0]:";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, r#"{"items":[]}"#);
}

// ============================================================================
// Root Arrays
// ============================================================================

#[test]
fn decode_root_inline_array() {
    let toon = "[3]: 1,2,3";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, "[1,2,3]");
}

#[test]
fn decode_root_mixed_array() {
    let toon = "[3]:\n  - hello\n  - [2]: 1,2\n  - name: Alice\n    age: 30";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, r#"["hello",[1,2],{"name":"Alice","age":30}]"#);
}

// ============================================================================
// Tabular Arrays
// ============================================================================

#[test]
fn decode_tabular_array_basic() {
    let toon = "users[2]{id,name,active}:\n  1,Alice,true\n  2,Bob,false";
    let json = decode(toon).unwrap();
    assert_json_eq(
        &json,
        r#"{"users":[{"id":1,"name":"Alice","active":true},{"id":2,"name":"Bob","active":false}]}"#,
    );
}

#[test]
fn decode_tabular_with_quoted_cell() {
    let toon = "items[2]{name,id}:\n  \"a,b\",1\n  c,2";
    let json = decode(toon).unwrap();
    assert_json_eq(
        &json,
        r#"{"items":[{"name":"a,b","id":1},{"name":"c","id":2}]}"#,
    );
}

#[test]
fn decode_tabular_single_row() {
    let toon = "data[1]{x,y}:\n  10,20";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, r#"{"data":[{"x":10,"y":20}]}"#);
}

#[test]
fn decode_tabular_preserves_field_order() {
    let toon = "items[2]{z,a}:\n  \"1\",\"2\"\n  \"3\",\"4\"";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, r#"{"items":[{"z":"1","a":"2"},{"z":"3","a":"4"}]}"#);
}

#[test]
fn decode_tabular_with_null() {
    let toon = "rows[2]{a,b}:\n  1,null\n  null,2";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, r#"{"rows":[{"a":1,"b":null},{"a":null,"b":2}]}"#);
}

// ============================================================================
// Mixed / Expanded Arrays (List Items)
// ============================================================================

#[test]
fn decode_mixed_array_primitives() {
    let toon = "items[2]:\n  - hello\n  - 42";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, r#"{"items":["hello",42]}"#);
}

#[test]
fn decode_mixed_array_objects() {
    let toon = "items[2]:\n  - name: Alice\n    age: 30\n  - name: Bob\n    age: 25";
    let json = decode(toon).unwrap();
    assert_json_eq(
        &json,
        r#"{"items":[{"name":"Alice","age":30},{"name":"Bob","age":25}]}"#,
    );
}

#[test]
fn decode_mixed_array_heterogeneous() {
    // Mix of primitives, objects, and arrays in a single list
    let toon = "data[3]:\n  - hello\n  - name: test\n  - [2]: 1,2";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, r#"{"data":["hello",{"name":"test"},[1,2]]}"#);
}

#[test]
fn decode_list_item_with_nested_object() {
    let toon =
        "items[1]:\n  - name: Alice\n    address:\n      city: Portland\n      zip: \"97201\"";
    let json = decode(toon).unwrap();
    assert_json_eq(
        &json,
        r#"{"items":[{"name":"Alice","address":{"city":"Portland","zip":"97201"}}]}"#,
    );
}

#[test]
fn decode_list_item_with_array_field() {
    let toon = "items[1]:\n  - name: Alice\n    tags[2]: admin,user";
    let json = decode(toon).unwrap();
    assert_json_eq(
        &json,
        r#"{"items":[{"name":"Alice","tags":["admin","user"]}]}"#,
    );
}

// ============================================================================
// String Value Type Inference
// ============================================================================

#[test]
fn decode_unquoted_value_as_string() {
    // Values that don't parse as number/bool/null should be strings
    let toon = "name: hello_world";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, r#"{"name":"hello_world"}"#);
}

#[test]
fn decode_integer_value() {
    let toon = "count: 42";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, r#"{"count":42}"#);
}

#[test]
fn decode_float_value() {
    let toon = "ratio: 3.14";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, r#"{"ratio":3.14}"#);
}

#[test]
fn decode_bool_value() {
    let toon = "active: true";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, r#"{"active":true}"#);
}

#[test]
fn decode_null_value() {
    let toon = "email: null";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, r#"{"email":null}"#);
}

#[test]
fn decode_quoted_number_as_string() {
    // "42" quoted means it's a string, not a number
    let toon = "code: \"42\"";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, r#"{"code":"42"}"#);
}

#[test]
fn decode_quoted_bool_as_string() {
    let toon = "label: \"true\"";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, r#"{"label":"true"}"#);
}

#[test]
fn decode_quoted_null_as_string() {
    let toon = "val: \"null\"";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, r#"{"val":"null"}"#);
}

// ============================================================================
// Escape Sequences in Values
// ============================================================================

#[test]
fn decode_string_with_escaped_newline() {
    let toon = "msg: \"line1\\nline2\"";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, r#"{"msg":"line1\nline2"}"#);
}

#[test]
fn decode_string_with_escaped_tab() {
    let toon = "msg: \"col1\\tcol2\"";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, r#"{"msg":"col1\tcol2"}"#);
}

#[test]
fn decode_string_with_escaped_backslash() {
    let toon = "path: \"C:\\\\Users\"";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, r#"{"path":"C:\\Users"}"#);
}

#[test]
fn decode_string_with_escaped_quote() {
    let toon = "msg: \"say \\\"hi\\\"\"";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, r#"{"msg":"say \"hi\""}"#);
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn decode_object_with_leading_zero_string() {
    // "05" is a string that looks numeric — should be quoted in TOON, decoded as string
    let toon = "zip: \"05401\"";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, r#"{"zip":"05401"}"#);
}

#[test]
fn decode_object_with_hyphen_string() {
    let toon = "val: \"-not-a-number\"";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, r#"{"val":"-not-a-number"}"#);
}

#[test]
fn decode_single_field_object() {
    let toon = "x: 1";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, r#"{"x":1}"#);
}

#[test]
fn decode_object_many_types() {
    let toon = "str: hello\nint: 42\nfloat: 3.14\nbool: true\nnul: null";
    let json = decode(toon).unwrap();
    assert_json_eq(
        &json,
        r#"{"str":"hello","int":42,"float":3.14,"bool":true,"nul":null}"#,
    );
}

#[test]
fn decode_negative_float() {
    let toon = "val: -1.5";
    let json = decode(toon).unwrap();
    // Negative float should parse as number, but since it starts with '-' it's quoted in TOON
    // Actually the encoder quotes values starting with '-', so the TOON would be: val: "-1.5"
    // This tests that an unquoted -1.5 (if ever encountered) parses correctly
    assert_json_eq(&json, r#"{"val":-1.5}"#);
}

// ============================================================================
// Calendar-Realistic Tabular
// ============================================================================

#[test]
fn decode_calendar_events_tabular() {
    let toon = "summary: Team Standup\nstart: \"2024-01-15T10:00:00Z\"\nend: \"2024-01-15T10:30:00Z\"\nattendees[2]{email,name,status}:\n  alice@co.com,Alice,accepted\n  bob@co.com,Bob,tentative";
    let json = decode(toon).unwrap();
    let expected = r#"{"summary":"Team Standup","start":"2024-01-15T10:00:00Z","end":"2024-01-15T10:30:00Z","attendees":[{"email":"alice@co.com","name":"Alice","status":"accepted"},{"email":"bob@co.com","name":"Bob","status":"tentative"}]}"#;
    assert_json_eq(&json, expected);
}

// ============================================================================
// Array of Arrays
// ============================================================================

#[test]
fn decode_array_of_arrays() {
    let toon = "matrix[2]:\n  - [3]: 1,2,3\n  - [3]: 4,5,6";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, r#"{"matrix":[[1,2,3],[4,5,6]]}"#);
}

// ============================================================================
// Objects with non-uniform arrays (not tabular → list items)
// ============================================================================

#[test]
fn decode_non_uniform_objects_in_array() {
    let toon = "items[2]:\n  - a: 1\n  - b: 2";
    let json = decode(toon).unwrap();
    assert_json_eq(&json, r#"{"items":[{"a":1},{"b":2}]}"#);
}
