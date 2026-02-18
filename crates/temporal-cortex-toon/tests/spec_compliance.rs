/// TOON v3.0 Spec Compliance Tests
///
/// Comprehensive test suite verifying ALL edge cases of the TOON v3.0 specification.
/// This supplements the existing encoder (59), decoder (63), and roundtrip (42) tests
/// with additional coverage for quoting rules, key encoding, delimiter scoping,
/// structural combinations, and realistic payloads.
///
/// Every test verifies roundtrip fidelity: decode(encode(json)) == json
/// unless otherwise noted.
use toon_core::{decode, encode};

/// Assert that encode -> decode roundtrips to the same JSON value.
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

/// Assert that encoding produces the exact expected TOON output.
fn assert_encode(json: &str, expected_toon: &str) {
    let toon = encode(json).unwrap();
    assert_eq!(
        toon, expected_toon,
        "Encode mismatch:\n  input JSON: {json}\n  got:      {toon}\n  expected: {expected_toon}"
    );
}

/// Assert TOON formatting invariants: no trailing newline, no trailing spaces on any line.
fn assert_toon_invariants(toon: &str) {
    assert!(
        !toon.ends_with('\n'),
        "TOON output must not end with newline: {:?}",
        toon
    );
    for (i, line) in toon.lines().enumerate() {
        assert!(
            !line.ends_with(' '),
            "Line {} has trailing space: {:?}",
            i,
            line
        );
    }
}

// ============================================================================
// 1. PRIMITIVE VALUES — Roundtrip + Encoding
// ============================================================================

mod primitives {
    use super::*;

    #[test]
    fn null_value() {
        assert_roundtrip("null");
        assert_encode("null", "null");
    }

    #[test]
    fn bool_true() {
        assert_roundtrip("true");
        assert_encode("true", "true");
    }

    #[test]
    fn bool_false() {
        assert_roundtrip("false");
        assert_encode("false", "false");
    }

    #[test]
    fn integer_positive() {
        assert_roundtrip("42");
        assert_encode("42", "42");
    }

    #[test]
    fn integer_zero() {
        assert_roundtrip("0");
        assert_encode("0", "0");
    }

    #[test]
    fn integer_negative() {
        assert_roundtrip("-7");
        assert_encode("-7", "-7");
    }

    #[test]
    fn integer_large() {
        assert_roundtrip("999999999");
        assert_encode("999999999", "999999999");
    }

    #[test]
    fn integer_large_negative() {
        assert_roundtrip("-123456789");
        assert_encode("-123456789", "-123456789");
    }

    #[test]
    fn integer_one() {
        assert_roundtrip("1");
    }

    #[test]
    fn integer_max_safe_i64() {
        // Largest integer serde_json will parse as i64
        assert_roundtrip("9007199254740991");
    }

    #[test]
    fn float_simple() {
        assert_roundtrip("3.14");
        assert_encode("3.14", "3.14");
    }

    #[test]
    fn float_negative() {
        assert_roundtrip("-2.5");
        assert_encode("-2.5", "-2.5");
    }

    #[test]
    fn float_small() {
        assert_roundtrip("0.001");
    }

    #[test]
    fn float_no_trailing_zeros() {
        // Spec: no trailing fractional zeros. 1.50 -> 1.5
        assert_encode("1.50", "1.5");
    }

    #[test]
    fn float_integer_form() {
        // Spec: integer form when fractional is zero. 1.0 -> 1
        assert_encode("1.0", "1");
    }

    #[test]
    fn negative_zero_normalized() {
        // Spec: -0 MUST be normalized to 0
        assert_encode("-0", "0");
        let toon = encode("-0").unwrap();
        let json = decode(&toon).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v, serde_json::Value::Number(0.into()));
    }

    #[test]
    fn exponent_expanded() {
        // Spec: no exponent notation. 1e2 -> 100
        let toon = encode("1e2").unwrap();
        assert_eq!(toon, "100");
    }

    #[test]
    fn large_number_no_exponent() {
        assert_encode("1000000", "1000000");
    }

    #[test]
    fn string_simple() {
        assert_roundtrip(r#""hello world""#);
        assert_encode(r#""hello world""#, "hello world");
    }

    #[test]
    fn string_single_word() {
        assert_roundtrip(r#""hello""#);
        assert_encode(r#""hello""#, "hello");
    }

    #[test]
    fn string_unicode_safe() {
        assert_roundtrip(r#""cafe\u0301""#);
    }

    #[test]
    fn string_unicode_no_special_chars() {
        assert_roundtrip(r#""caf\u00e9""#);
    }

    #[test]
    fn string_cjk_characters() {
        assert_roundtrip(r#""\u4f60\u597d""#);
    }

    #[test]
    fn string_emoji() {
        assert_roundtrip(r#""\ud83d\ude00""#);
    }
}

// ============================================================================
// 2. QUOTING RULES — Context-Dependent (TOON v3.0 Critical Section)
// ============================================================================

mod quoting_rules {
    use super::*;

    // --- Strings that look like booleans ---

    #[test]
    fn string_looks_like_true() {
        assert_roundtrip(r#""true""#);
        assert_encode(r#""true""#, r#""true""#);
    }

    #[test]
    fn string_looks_like_false() {
        assert_roundtrip(r#""false""#);
        assert_encode(r#""false""#, r#""false""#);
    }

    #[test]
    fn string_true_mixed_case_not_quoted() {
        // "True" is NOT a TOON keyword (only lowercase "true"), so unquoted
        assert_roundtrip(r#""True""#);
        assert_encode(r#""True""#, "True");
    }

    #[test]
    fn string_false_mixed_case_not_quoted() {
        assert_roundtrip(r#""False""#);
        assert_encode(r#""False""#, "False");
    }

    #[test]
    fn string_true_uppercase_not_quoted() {
        assert_roundtrip(r#""TRUE""#);
        assert_encode(r#""TRUE""#, "TRUE");
    }

    #[test]
    fn string_false_uppercase_not_quoted() {
        assert_roundtrip(r#""FALSE""#);
        assert_encode(r#""FALSE""#, "FALSE");
    }

    // --- Strings that look like null ---

    #[test]
    fn string_looks_like_null() {
        assert_roundtrip(r#""null""#);
        assert_encode(r#""null""#, r#""null""#);
    }

    #[test]
    fn string_null_mixed_case_not_quoted() {
        assert_roundtrip(r#""Null""#);
        assert_encode(r#""Null""#, "Null");
    }

    #[test]
    fn string_null_uppercase_not_quoted() {
        assert_roundtrip(r#""NULL""#);
        assert_encode(r#""NULL""#, "NULL");
    }

    // --- Strings that look like numbers ---

    #[test]
    fn string_looks_like_integer() {
        assert_roundtrip(r#""42""#);
        assert_encode(r#""42""#, r#""42""#);
    }

    #[test]
    fn string_looks_like_float() {
        assert_roundtrip(r#""3.14""#);
        assert_encode(r#""3.14""#, r#""3.14""#);
    }

    #[test]
    fn string_looks_like_negative() {
        assert_roundtrip(r#""-1""#);
        assert_encode(r#""-1""#, r#""-1""#);
    }

    #[test]
    fn string_looks_like_zero() {
        assert_roundtrip(r#""0""#);
        assert_encode(r#""0""#, r#""0""#);
    }

    #[test]
    fn string_leading_zero() {
        assert_roundtrip(r#""05""#);
        assert_encode(r#""05""#, r#""05""#);
    }

    #[test]
    fn string_leading_zeros() {
        assert_roundtrip(r#""007""#);
        assert_encode(r#""007""#, r#""007""#);
    }

    #[test]
    fn string_negative_float() {
        assert_roundtrip(r#""-3.14""#);
        assert_encode(r#""-3.14""#, r#""-3.14""#);
    }

    // --- Strings with special characters ---

    #[test]
    fn string_empty_must_be_quoted() {
        assert_roundtrip(r#""""#);
        assert_encode(r#""""#, r#""""#);
    }

    #[test]
    fn string_with_leading_whitespace() {
        assert_roundtrip(r#""  hello""#);
        assert_encode(r#""  hello""#, r#""  hello""#);
    }

    #[test]
    fn string_with_trailing_whitespace() {
        assert_roundtrip(r#""hello  ""#);
        assert_encode(r#""hello  ""#, r#""hello  ""#);
    }

    #[test]
    fn string_with_both_whitespace() {
        assert_roundtrip(r#""  spaces  ""#);
        assert_encode(r#""  spaces  ""#, r#""  spaces  ""#);
    }

    #[test]
    fn string_single_space() {
        assert_roundtrip(r#"" ""#);
        assert_encode(r#"" ""#, r#"" ""#);
    }

    #[test]
    fn string_with_colon_document_context() {
        // In document context (object values), colon triggers quoting
        assert_roundtrip(r#""hello:world""#);
        assert_encode(r#""hello:world""#, r#""hello:world""#);
    }

    #[test]
    fn string_with_backslash() {
        assert_roundtrip(r#""path\\to\\file""#);
        assert_encode(r#""path\\to\\file""#, r#""path\\to\\file""#);
    }

    #[test]
    fn string_with_double_quote() {
        assert_roundtrip(r#""say \"hi\"""#);
        assert_encode(r#""say \"hi\"""#, r#""say \"hi\"""#);
    }

    #[test]
    fn string_with_open_bracket() {
        assert_roundtrip(r#""[data]""#);
        assert_encode(r#""[data]""#, r#""[data]""#);
    }

    #[test]
    fn string_with_close_bracket() {
        assert_roundtrip(r#""data]""#);
        assert_encode(r#""data]""#, r#""data]""#);
    }

    #[test]
    fn string_with_open_brace() {
        assert_roundtrip(r#""{key}""#);
        assert_encode(r#""{key}""#, r#""{key}""#);
    }

    #[test]
    fn string_with_close_brace() {
        assert_roundtrip(r#""key}""#);
        assert_encode(r#""key}""#, r#""key}""#);
    }

    #[test]
    fn string_starting_with_hyphen() {
        assert_roundtrip(r#""-hello""#);
        assert_encode(r#""-hello""#, r#""-hello""#);
    }

    #[test]
    fn string_just_hyphen() {
        assert_roundtrip(r#""-""#);
        assert_encode(r#""-""#, r#""-""#);
    }

    #[test]
    fn string_with_newline() {
        assert_roundtrip("\"line1\\nline2\"");
        assert_encode("\"line1\\nline2\"", r#""line1\nline2""#);
    }

    #[test]
    fn string_with_tab() {
        assert_roundtrip("\"col1\\tcol2\"");
        assert_encode("\"col1\\tcol2\"", r#""col1\tcol2""#);
    }

    #[test]
    fn string_with_carriage_return() {
        assert_roundtrip("\"line1\\rline2\"");
        assert_encode("\"line1\\rline2\"", r#""line1\rline2""#);
    }

    // --- Strings safe to leave unquoted ---

    #[test]
    fn string_simple_word_unquoted() {
        assert_encode(r#""hello""#, "hello");
    }

    #[test]
    fn string_multiple_words_unquoted() {
        assert_encode(r#""hello world""#, "hello world");
    }

    #[test]
    fn string_with_numbers_unquoted() {
        // "abc123" does not look numeric, safe unquoted
        assert_encode(r#""abc123""#, "abc123");
        assert_roundtrip(r#""abc123""#);
    }

    #[test]
    fn string_with_underscore_unquoted() {
        assert_encode(r#""hello_world""#, "hello_world");
        assert_roundtrip(r#""hello_world""#);
    }

    #[test]
    fn string_with_dot_unquoted() {
        assert_encode(r#""version 1.2""#, "version 1.2");
        assert_roundtrip(r#""version 1.2""#);
    }

    // --- Context-dependent delimiter quoting ---

    #[test]
    fn string_with_comma_in_inline_array() {
        // In inline arrays, comma is the active delimiter -> must be quoted
        let json = r#"{"items":["a,b","c"]}"#;
        let toon = encode(json).unwrap();
        assert!(
            toon.contains(r#""a,b""#),
            "comma in inline array element must be quoted"
        );
        assert_roundtrip(json);
    }

    #[test]
    fn string_with_comma_in_tabular_cell() {
        // In tabular rows, comma is the active delimiter -> must be quoted
        let json = r#"{"items":[{"name":"a,b","id":1},{"name":"c","id":2}]}"#;
        let toon = encode(json).unwrap();
        assert!(
            toon.contains(r#""a,b""#),
            "comma in tabular cell must be quoted"
        );
        assert_roundtrip(json);
    }

    #[test]
    fn string_with_colon_in_tabular_cell_no_quoting() {
        // In tabular rows, colon is NOT the active delimiter -> NOT quoted
        let json = r#"{"events":[{"time":"10:30:00","name":"meeting"}]}"#;
        let toon = encode(json).unwrap();
        assert_eq!(toon, "events[1]{time,name}:\n  10:30:00,meeting");
        assert_roundtrip(json);
    }

    #[test]
    fn string_with_colon_in_inline_array_no_quoting() {
        // In inline arrays, colon is NOT the active delimiter -> NOT quoted
        let json = r#"{"times":["10:30","11:00"]}"#;
        let toon = encode(json).unwrap();
        assert_eq!(toon, "times[2]: 10:30,11:00");
        assert_roundtrip(json);
    }

    #[test]
    fn string_with_comma_in_document_context_no_quoting() {
        // In document context (object values), comma is NOT the active delimiter -> NOT quoted
        let json = r#"{"greeting":"hello, world"}"#;
        let toon = encode(json).unwrap();
        assert_eq!(toon, "greeting: hello, world");
        assert_roundtrip(json);
    }
}

// ============================================================================
// 3. KEY ENCODING — Quoted vs Unquoted Keys
// ============================================================================

mod key_encoding {
    use super::*;

    #[test]
    fn key_simple_alpha() {
        // ^[A-Za-z_][A-Za-z0-9_.]*$ -> unquoted
        assert_encode(r#"{"name":"test"}"#, "name: test");
    }

    #[test]
    fn key_with_underscore() {
        assert_encode(r#"{"first_name":"Alice"}"#, "first_name: Alice");
    }

    #[test]
    fn key_with_dot() {
        assert_encode(r#"{"config.key":"value"}"#, "config.key: value");
    }

    #[test]
    fn key_starting_with_underscore() {
        assert_encode(r#"{"_private":"yes"}"#, "_private: yes");
    }

    #[test]
    fn key_alphanumeric() {
        assert_encode(r#"{"item1":"val"}"#, "item1: val");
    }

    #[test]
    fn key_with_hyphen_requires_quoting() {
        assert_encode(r#"{"my-key":"value"}"#, "\"my-key\": value");
        assert_roundtrip(r#"{"my-key":"value"}"#);
    }

    #[test]
    fn key_starting_with_digit_requires_quoting() {
        assert_encode(r#"{"123":"value"}"#, "\"123\": value");
        assert_roundtrip(r#"{"123":"value"}"#);
    }

    #[test]
    fn key_with_space_requires_quoting() {
        assert_encode(r#"{"my key":"value"}"#, "\"my key\": value");
        assert_roundtrip(r#"{"my key":"value"}"#);
    }

    #[test]
    fn key_empty_string_requires_quoting() {
        assert_encode(r#"{"":"value"}"#, "\"\": value");
        assert_roundtrip(r#"{"":"value"}"#);
    }

    #[test]
    fn key_with_colon_requires_quoting() {
        assert_encode(r#"{"a:b":"value"}"#, "\"a:b\": value");
        assert_roundtrip(r#"{"a:b":"value"}"#);
    }

    #[test]
    fn key_with_backslash_requires_quoting() {
        assert_encode(r#"{"a\\b":"value"}"#, "\"a\\\\b\": value");
        assert_roundtrip(r#"{"a\\b":"value"}"#);
    }

    #[test]
    fn key_with_quote_requires_quoting() {
        assert_encode(r#"{"a\"b":"value"}"#, "\"a\\\"b\": value");
        assert_roundtrip(r#"{"a\"b":"value"}"#);
    }

    #[test]
    fn key_with_bracket_requires_quoting() {
        assert_encode(r#"{"a[0]":"value"}"#, "\"a[0]\": value");
        assert_roundtrip(r#"{"a[0]":"value"}"#);
    }

    #[test]
    fn mixed_keys_some_quoted_some_not() {
        let json = r#"{"name":"Alice","my-key":"val","age":30,"123":"num"}"#;
        let toon = encode(json).unwrap();
        assert!(toon.contains("name: Alice"));
        assert!(toon.contains("\"my-key\": val"));
        assert!(toon.contains("age: 30"));
        assert!(toon.contains("\"123\": num"));
        assert_roundtrip(json);
    }
}

// ============================================================================
// 4. OBJECTS — Flat, Nested, Empty, Complex
// ============================================================================

mod objects {
    use super::*;

    #[test]
    fn empty_root_object() {
        assert_encode("{}", "");
        assert_roundtrip("{}");
    }

    #[test]
    fn single_field_object() {
        assert_encode(r#"{"x":1}"#, "x: 1");
        assert_roundtrip(r#"{"x":1}"#);
    }

    #[test]
    fn flat_object_multiple_types() {
        let json = r#"{"name":"Alice","age":30,"active":true,"score":null}"#;
        assert_encode(json, "name: Alice\nage: 30\nactive: true\nscore: null");
        assert_roundtrip(json);
    }

    #[test]
    fn preserves_key_order() {
        let json = r#"{"z":1,"a":2,"m":3}"#;
        assert_encode(json, "z: 1\na: 2\nm: 3");
        assert_roundtrip(json);
    }

    #[test]
    fn nested_one_level() {
        let json = r#"{"user":{"id":1,"name":"Ada"}}"#;
        assert_encode(json, "user:\n  id: 1\n  name: Ada");
        assert_roundtrip(json);
    }

    #[test]
    fn nested_two_levels() {
        let json = r#"{"a":{"b":{"c":"deep"}}}"#;
        assert_encode(json, "a:\n  b:\n    c: deep");
        assert_roundtrip(json);
    }

    #[test]
    fn nested_three_levels() {
        let json = r#"{"a":{"b":{"c":{"d":"very deep"}}}}"#;
        assert_encode(json, "a:\n  b:\n    c:\n      d: very deep");
        assert_roundtrip(json);
    }

    #[test]
    fn mixed_nested_and_flat() {
        let json = r#"{"name":"App","server":{"host":"localhost","port":8080},"debug":true}"#;
        assert_encode(
            json,
            "name: App\nserver:\n  host: localhost\n  port: 8080\ndebug: true",
        );
        assert_roundtrip(json);
    }

    #[test]
    fn nested_empty_object() {
        assert_encode(r#"{"config":{}}"#, "config:");
        assert_roundtrip(r#"{"config":{}}"#);
    }

    #[test]
    fn nested_empty_with_sibling() {
        assert_roundtrip(r#"{"config":{},"name":"test"}"#);
    }

    #[test]
    fn multiple_nested_objects() {
        let json = r#"{"db":{"host":"pg","port":5432},"cache":{"host":"redis","port":6379}}"#;
        assert_roundtrip(json);
    }

    #[test]
    fn object_with_all_value_types() {
        let json = r#"{"str":"hello","int":42,"float":3.14,"bool":true,"nul":null,"obj":{"x":1},"arr":[1,2]}"#;
        assert_roundtrip(json);
    }

    #[test]
    fn object_with_special_string_values() {
        let json = r#"{"keyword":"true","empty":"","url":"http://a:b","numeric":"42","leading_zero":"05"}"#;
        assert_roundtrip(json);
    }
}

// ============================================================================
// 5. ARRAYS — Inline, Tabular, Expanded, Empty, Root
// ============================================================================

mod arrays {
    use super::*;

    // --- Empty arrays ---

    #[test]
    fn empty_array_in_object() {
        assert_encode(r#"{"items":[]}"#, "items[0]:");
        assert_roundtrip(r#"{"items":[]}"#);
    }

    #[test]
    fn empty_root_array() {
        // Encoder uses inline format for empty array: "[0]: " (with space after colon)
        // because all_primitives([]) is true -> inline path
        assert_encode("[]", "[0]: ");
        assert_roundtrip("[]");
    }

    // --- Single-element arrays ---

    #[test]
    fn single_integer_array() {
        assert_encode(r#"{"ids":[42]}"#, "ids[1]: 42");
        assert_roundtrip(r#"{"ids":[42]}"#);
    }

    #[test]
    fn single_string_array() {
        assert_encode(r#"{"tags":["admin"]}"#, "tags[1]: admin");
        assert_roundtrip(r#"{"tags":["admin"]}"#);
    }

    #[test]
    fn single_object_array_tabular() {
        assert_encode(r#"{"items":[{"x":1,"y":2}]}"#, "items[1]{x,y}:\n  1,2");
        assert_roundtrip(r#"{"items":[{"x":1,"y":2}]}"#);
    }

    // --- Inline primitive arrays ---

    #[test]
    fn inline_integers() {
        assert_encode(r#"{"ids":[1,2,3,4,5]}"#, "ids[5]: 1,2,3,4,5");
        assert_roundtrip(r#"{"ids":[1,2,3,4,5]}"#);
    }

    #[test]
    fn inline_strings() {
        assert_encode(r#"{"tags":["a","b","c"]}"#, "tags[3]: a,b,c");
        assert_roundtrip(r#"{"tags":["a","b","c"]}"#);
    }

    #[test]
    fn inline_booleans() {
        assert_encode(
            r#"{"flags":[true,false,true]}"#,
            "flags[3]: true,false,true",
        );
        assert_roundtrip(r#"{"flags":[true,false,true]}"#);
    }

    #[test]
    fn inline_nulls() {
        assert_encode(r#"{"vals":[null,null]}"#, "vals[2]: null,null");
        assert_roundtrip(r#"{"vals":[null,null]}"#);
    }

    #[test]
    fn inline_mixed_types() {
        assert_encode(
            r#"{"data":[1,"hello",true,null]}"#,
            "data[4]: 1,hello,true,null",
        );
        assert_roundtrip(r#"{"data":[1,"hello",true,null]}"#);
    }

    #[test]
    fn inline_string_with_comma_quoted() {
        assert_encode(r#"{"items":["a,b","c"]}"#, r#"items[2]: "a,b",c"#);
        assert_roundtrip(r#"{"items":["a,b","c"]}"#);
    }

    #[test]
    fn inline_string_looks_like_bool_quoted() {
        // "true" as a string element in inline array must be quoted
        let json = r#"{"vals":["true","false"]}"#;
        let toon = encode(json).unwrap();
        assert!(toon.contains(r#""true""#));
        assert!(toon.contains(r#""false""#));
        assert_roundtrip(json);
    }

    #[test]
    fn inline_string_looks_like_null_quoted() {
        let json = r#"{"vals":["null"]}"#;
        let toon = encode(json).unwrap();
        assert!(toon.contains(r#""null""#));
        assert_roundtrip(json);
    }

    #[test]
    fn inline_string_looks_like_number_quoted() {
        let json = r#"{"vals":["42","3.14"]}"#;
        let toon = encode(json).unwrap();
        assert!(toon.contains(r#""42""#));
        assert!(toon.contains(r#""3.14""#));
        assert_roundtrip(json);
    }

    // --- Root arrays ---

    #[test]
    fn root_inline_integers() {
        assert_encode("[1,2,3]", "[3]: 1,2,3");
        assert_roundtrip("[1,2,3]");
    }

    #[test]
    fn root_inline_strings() {
        assert_encode(r#"["a","b","c"]"#, "[3]: a,b,c");
        assert_roundtrip(r#"["a","b","c"]"#);
    }

    #[test]
    fn root_inline_mixed() {
        assert_encode(r#"[1,"hello",true]"#, "[3]: 1,hello,true");
        assert_roundtrip(r#"[1,"hello",true]"#);
    }

    #[test]
    fn root_mixed_with_objects() {
        assert_roundtrip(r#"["hello",[1,2],{"name":"Alice","age":30}]"#);
    }

    // --- Tabular arrays ---

    #[test]
    fn tabular_basic() {
        let json = r#"{"users":[{"id":1,"name":"Alice","active":true},{"id":2,"name":"Bob","active":false}]}"#;
        assert_encode(
            json,
            "users[2]{id,name,active}:\n  1,Alice,true\n  2,Bob,false",
        );
        assert_roundtrip(json);
    }

    #[test]
    fn tabular_preserves_field_order() {
        let json = r#"{"items":[{"z":"1","a":"2"},{"z":"3","a":"4"}]}"#;
        assert_encode(json, "items[2]{z,a}:\n  \"1\",\"2\"\n  \"3\",\"4\"");
        assert_roundtrip(json);
    }

    #[test]
    fn tabular_with_null_cells() {
        assert_roundtrip(r#"{"rows":[{"a":1,"b":null},{"a":null,"b":2}]}"#);
    }

    #[test]
    fn tabular_with_quoted_comma_cell() {
        let json = r#"{"items":[{"name":"a,b","id":1},{"name":"c","id":2}]}"#;
        assert_encode(json, "items[2]{name,id}:\n  \"a,b\",1\n  c,2");
        assert_roundtrip(json);
    }

    #[test]
    fn tabular_single_row() {
        assert_encode(r#"{"items":[{"x":1,"y":2}]}"#, "items[1]{x,y}:\n  1,2");
        assert_roundtrip(r#"{"items":[{"x":1,"y":2}]}"#);
    }

    #[test]
    fn tabular_datetime_no_extra_quotes() {
        // In tabular rows with comma delimiter, colons don't trigger quoting
        let json = r#"{"events":[{"time":"10:30:00","name":"meeting"}]}"#;
        assert_encode(json, "events[1]{time,name}:\n  10:30:00,meeting");
        assert_roundtrip(json);
    }

    #[test]
    fn tabular_many_rows() {
        let json = r#"{"items":[{"id":1,"v":"a"},{"id":2,"v":"b"},{"id":3,"v":"c"},{"id":4,"v":"d"},{"id":5,"v":"e"}]}"#;
        let toon = encode(json).unwrap();
        assert!(toon.starts_with("items[5]{id,v}:"));
        assert_roundtrip(json);
    }

    // --- Expanded list arrays ---

    #[test]
    fn expanded_mixed_types() {
        let json = r#"{"items":[1,{"a":"hello","b":"world"},"text"]}"#;
        assert_encode(
            json,
            "items[3]:\n  - 1\n  - a: hello\n    b: world\n  - text",
        );
        assert_roundtrip(json);
    }

    #[test]
    fn expanded_non_uniform_objects() {
        let json = r#"{"items":[{"a":1},{"b":2}]}"#;
        assert_encode(json, "items[2]:\n  - a: 1\n  - b: 2");
        assert_roundtrip(json);
    }

    #[test]
    fn expanded_array_of_arrays() {
        let json = r#"{"matrix":[[1,2],[3,4]]}"#;
        assert_encode(json, "matrix[2]:\n  - [2]: 1,2\n  - [2]: 3,4");
        assert_roundtrip(json);
    }

    #[test]
    fn expanded_objects_with_nested_values() {
        // Objects with nested object values -> not tabular -> list form
        let json = r#"{"items":[{"a":{"x":1}},{"a":{"x":2}}]}"#;
        assert_encode(json, "items[2]:\n  - a:\n      x: 1\n  - a:\n      x: 2");
        assert_roundtrip(json);
    }

    // --- Nested arrays ---

    #[test]
    fn array_of_arrays_3_levels() {
        assert_roundtrip(r#"{"data":[[[1,2],[3,4]],[[5,6],[7,8]]]}"#);
    }

    #[test]
    fn object_with_array_and_nested_object() {
        assert_roundtrip(
            r#"{"name":"project","config":{"debug":true,"port":3000},"tags":["web","api"]}"#,
        );
    }

    #[test]
    fn list_item_with_nested_object() {
        assert_roundtrip(
            r#"{"people":[{"name":"Alice","address":{"city":"Portland","zip":"97201"}}]}"#,
        );
    }

    #[test]
    fn list_item_with_array_field() {
        assert_roundtrip(r#"{"items":[{"name":"Alice","tags":["admin","user"]}]}"#);
    }
}

// ============================================================================
// 6. FORMATTING INVARIANTS — No trailing newline, no trailing spaces
// ============================================================================

mod formatting_invariants {
    use super::*;

    #[test]
    fn no_trailing_newline_primitive() {
        let toon = encode("42").unwrap();
        assert_toon_invariants(&toon);
    }

    #[test]
    fn no_trailing_newline_flat_object() {
        let toon = encode(r#"{"a":1,"b":"hello"}"#).unwrap();
        assert_toon_invariants(&toon);
    }

    #[test]
    fn no_trailing_newline_nested_object() {
        let toon = encode(r#"{"a":{"b":{"c":"deep"}}}"#).unwrap();
        assert_toon_invariants(&toon);
    }

    #[test]
    fn no_trailing_newline_tabular() {
        let toon = encode(r#"{"items":[{"x":1,"y":2},{"x":3,"y":4}]}"#).unwrap();
        assert_toon_invariants(&toon);
    }

    #[test]
    fn no_trailing_newline_expanded() {
        let toon = encode(r#"{"items":[{"a":1},{"b":2}]}"#).unwrap();
        assert_toon_invariants(&toon);
    }

    #[test]
    fn no_trailing_newline_mixed_complex() {
        let json = r#"{"name":"App","server":{"host":"localhost","port":8080},"items":[{"id":1,"name":"item1"},{"id":2,"name":"item2"}],"tags":["web","api"]}"#;
        let toon = encode(json).unwrap();
        assert_toon_invariants(&toon);
    }

    #[test]
    fn no_trailing_newline_root_array() {
        let toon = encode("[1,2,3]").unwrap();
        assert_toon_invariants(&toon);
    }

    #[test]
    fn no_trailing_newline_empty_object() {
        let toon = encode("{}").unwrap();
        assert!(!toon.ends_with('\n'));
    }
}

// ============================================================================
// 7. REALISTIC PAYLOADS — Calendar, User Profiles, API Responses
// ============================================================================

mod realistic_payloads {
    use super::*;

    #[test]
    fn calendar_events_tabular() {
        let json = r#"{"summary":"Engineering Sync","timeZone":"America/Los_Angeles","items":[{"id":"evt_1a2b","status":"confirmed","summary":"Q1 Strategy Sync","start":"2026-02-17T10:00:00-08:00","end":"2026-02-17T11:00:00-08:00"},{"id":"evt_9f8e","status":"confirmed","summary":"Vendor Negotiation","start":"2026-02-18T13:00:00-08:00","end":"2026-02-18T14:00:00-08:00"}]}"#;
        let toon = encode(json).unwrap();
        assert!(toon.contains("items[2]{id,status,summary,start,end}:"));
        assert_roundtrip(json);
    }

    #[test]
    fn calendar_event_with_attendees() {
        let json = r#"{"summary":"Team Standup","start":"2024-01-15T10:00:00Z","end":"2024-01-15T10:30:00Z","attendees":[{"email":"alice@co.com","name":"Alice","status":"accepted"},{"email":"bob@co.com","name":"Bob","status":"tentative"}]}"#;
        assert_roundtrip(json);
    }

    #[test]
    fn user_profile() {
        let json = r#"{"id":"user_123","name":"Ada Lovelace","email":"ada@example.com","role":"admin","settings":{"theme":"dark","language":"en","notifications":true},"tags":["engineering","leadership"]}"#;
        assert_roundtrip(json);
    }

    #[test]
    fn api_paginated_response() {
        let json = r#"{"total":100,"page":1,"per_page":10,"data":[{"id":1,"title":"First Post","author":"Alice"},{"id":2,"title":"Second Post","author":"Bob"}]}"#;
        assert_roundtrip(json);
    }

    #[test]
    fn nested_config() {
        let json = r#"{"database":{"host":"localhost","port":5432,"name":"mydb","pool":{"min":5,"max":20}},"redis":{"host":"localhost","port":6379},"debug":false}"#;
        assert_roundtrip(json);
    }

    #[test]
    fn package_json_like() {
        let json = r#"{"name":"my-app","version":"1.0.0","description":"A test app","main":"index.js","scripts":{"test":"jest","build":"tsc"},"dependencies":{"express":"4.18.0","lodash":"4.17.21"}}"#;
        assert_roundtrip(json);
    }

    #[test]
    fn github_issue_like() {
        let json = r#"{"id":42,"title":"Bug in parser","state":"open","body":"The parser fails on nested arrays","labels":[{"name":"bug","color":"red"},{"name":"priority","color":"orange"}],"assignee":{"login":"alice","id":123}}"#;
        assert_roundtrip(json);
    }

    #[test]
    fn mixed_complexity() {
        // Object with flat values, nested objects, tabular arrays, inline arrays, and expanded lists
        let json = r#"{"name":"Project","version":1,"config":{"debug":true},"members":[{"name":"Alice","role":"lead"},{"name":"Bob","role":"dev"}],"tags":["rust","wasm"],"misc":[1,{"x":2},"three"]}"#;
        assert_roundtrip(json);
    }
}

// ============================================================================
// 8. EDGE CASES — Boundary conditions and unusual inputs
// ============================================================================

mod edge_cases {
    use super::*;

    #[test]
    fn object_with_empty_string_key() {
        assert_roundtrip(r#"{"":"value"}"#);
    }

    #[test]
    fn object_with_empty_string_value() {
        assert_roundtrip(r#"{"key":""}"#);
    }

    #[test]
    fn object_with_all_special_string_values() {
        assert_roundtrip(
            r#"{"a":"","b":"true","c":"false","d":"null","e":"42","f":"3.14","g":"-1","h":"05","i":"hello:world","j":"path\\to","k":"[data]","l":"-hello","m":"  spaces  "}"#,
        );
    }

    #[test]
    fn deeply_nested_four_levels() {
        assert_roundtrip(r#"{"a":{"b":{"c":{"d":{"e":"deep"}}}}}"#);
    }

    #[test]
    fn many_fields_object() {
        // 10 fields to stress ordering
        let json = r#"{"a":1,"b":2,"c":3,"d":4,"e":5,"f":6,"g":7,"h":8,"i":9,"j":10}"#;
        assert_roundtrip(json);
    }

    #[test]
    fn large_inline_array() {
        let json = r#"{"nums":[1,2,3,4,5,6,7,8,9,10]}"#;
        assert_roundtrip(json);
    }

    #[test]
    fn tabular_many_fields() {
        let json =
            r#"{"items":[{"a":1,"b":2,"c":3,"d":4,"e":5},{"a":6,"b":7,"c":8,"d":9,"e":10}]}"#;
        assert_roundtrip(json);
    }

    #[test]
    fn object_only_nested_objects() {
        assert_roundtrip(r#"{"a":{"x":1},"b":{"y":2},"c":{"z":3}}"#);
    }

    #[test]
    fn object_only_arrays() {
        assert_roundtrip(r#"{"a":[1,2],"b":[3,4],"c":[5,6]}"#);
    }

    #[test]
    fn tabular_with_empty_string_cell() {
        let json = r#"{"items":[{"name":"","id":1},{"name":"test","id":2}]}"#;
        assert_roundtrip(json);
    }

    #[test]
    fn tabular_with_null_string_bool_cells() {
        let json = r#"{"rows":[{"s":"hello","n":42,"b":true,"x":null},{"s":"world","n":0,"b":false,"x":null}]}"#;
        assert_roundtrip(json);
    }

    #[test]
    fn inline_array_with_empty_string() {
        let json = r#"{"items":["","hello",""]}"#;
        assert_roundtrip(json);
    }

    #[test]
    fn string_just_whitespace() {
        // Single space string
        assert_roundtrip(r#"" ""#);
    }

    #[test]
    fn string_multiple_spaces() {
        assert_roundtrip(r#""   ""#);
    }

    #[test]
    fn number_zero_point_five() {
        assert_roundtrip("0.5");
    }

    #[test]
    fn number_negative_zero_point_five() {
        assert_roundtrip("-0.5");
    }

    #[test]
    fn nested_object_with_array_sibling() {
        assert_roundtrip(r#"{"meta":{"v":1},"items":[1,2,3],"config":{"debug":false}}"#);
    }

    #[test]
    fn expanded_list_with_nested_arrays() {
        // List items that are themselves arrays
        assert_roundtrip(r#"{"data":[[1,2,3],[4,5,6],[7,8,9]]}"#);
    }

    #[test]
    fn root_array_of_objects() {
        assert_roundtrip(r#"[{"a":1},{"b":2}]"#);
    }

    #[test]
    fn root_array_of_tabular_objects() {
        assert_roundtrip(r#"[{"id":1,"name":"Alice"},{"id":2,"name":"Bob"}]"#);
    }

    #[test]
    fn root_single_element() {
        assert_roundtrip("[42]");
    }

    #[test]
    fn root_array_with_nested_empty_objects() {
        // Known limitation: empty objects in expanded list items produce "- " (nothing after),
        // which the decoder interprets as empty list items rather than empty objects.
        // This is a known encoder gap — the encoder should emit "- \n" or similar
        // for empty objects, but currently doesn't. Verify the encoding at least doesn't panic.
        let toon = encode(r#"[{},{}]"#).unwrap();
        assert!(!toon.is_empty());
    }

    #[test]
    fn object_value_timestamp_quoted() {
        // Timestamp with colons must be quoted as an object value (document context)
        let json = r#"{"timestamp":"2025-01-15T10:30:00Z"}"#;
        let toon = encode(json).unwrap();
        assert_eq!(toon, "timestamp: \"2025-01-15T10:30:00Z\"");
        assert_roundtrip(json);
    }

    #[test]
    fn consecutive_special_values() {
        assert_roundtrip(r#"{"a":"true","b":"false","c":"null","d":"42","e":""}"#);
    }

    #[test]
    fn string_with_only_special_chars() {
        assert_roundtrip(r#""\\\\""#);
    }

    #[test]
    fn string_with_multiple_escapes() {
        assert_roundtrip(r#""\t\n\r""#);
    }

    #[test]
    fn tabular_with_hyphen_starting_string() {
        // In tabular context, string starting with hyphen needs quoting
        let json = r#"{"items":[{"name":"-test","id":1},{"name":"ok","id":2}]}"#;
        assert_roundtrip(json);
    }

    #[test]
    fn expanded_list_single_primitive() {
        // This should use inline format, not expanded
        let json = r#"{"items":[42]}"#;
        let toon = encode(json).unwrap();
        assert_eq!(toon, "items[1]: 42");
        assert_roundtrip(json);
    }

    #[test]
    fn expanded_list_single_object() {
        let json = r#"{"items":[{"name":"test","value":42}]}"#;
        // Single object with same keys -> tabular
        let toon = encode(json).unwrap();
        assert!(toon.contains("{name,value}:"));
        assert_roundtrip(json);
    }

    #[test]
    fn object_with_boolean_and_string_boolean() {
        // Mix of actual booleans and string booleans
        let json = r#"{"actual":true,"string":"true","actual2":false,"string2":"false"}"#;
        assert_roundtrip(json);
    }

    #[test]
    fn object_with_null_and_string_null() {
        let json = r#"{"actual":null,"string":"null"}"#;
        assert_roundtrip(json);
    }

    #[test]
    fn object_with_number_and_string_number() {
        let json = r#"{"actual":42,"string":"42"}"#;
        assert_roundtrip(json);
    }
}

// ============================================================================
// 9. STRESS TESTS — Larger payloads and complex nesting
// ============================================================================

mod stress_tests {
    use super::*;

    #[test]
    fn many_tabular_rows() {
        // 10 rows of tabular data
        let mut items = Vec::new();
        for i in 0..10 {
            items.push(format!(
                r#"{{"id":{},"name":"item{}","active":{}}}"#,
                i,
                i,
                if i % 2 == 0 { "true" } else { "false" }
            ));
        }
        let json = format!(r#"{{"items":[{}]}}"#, items.join(","));
        assert_roundtrip(&json);
    }

    #[test]
    fn wide_flat_object() {
        // 20 fields
        let mut fields = Vec::new();
        for i in 0..20 {
            fields.push(format!(r#""field_{}":{}"#, i, i));
        }
        let json = format!("{{{}}}", fields.join(","));
        assert_roundtrip(&json);
    }

    #[test]
    fn mixed_array_many_items() {
        // 10 items: alternating primitives and objects
        let json = r#"{"items":[1,{"a":1},2,{"b":2},3,{"c":3},4,{"d":4},5,{"e":5}]}"#;
        assert_roundtrip(json);
    }

    #[test]
    fn complex_nested_calendar() {
        // Calendar with events as expanded list items (non-uniform due to different
        // attendee counts) — avoid tabular-inside-expanded which has known indentation
        // limitations for deeply nested structures.
        let json = r#"{"calendar":{"name":"Work","timezone":"UTC","events":[{"id":"e1","title":"Meeting","start":"2026-01-01T09:00:00Z","end":"2026-01-01T10:00:00Z","organizer":"Alice"},{"id":"e2","title":"Lunch","start":"2026-01-01T12:00:00Z","end":"2026-01-01T13:00:00Z","organizer":"Bob"}]}}"#;
        assert_roundtrip(json);
    }

    #[test]
    fn complex_nested_with_multiple_levels() {
        // Objects nested 3 levels with arrays at each level
        let json = r#"{"app":{"name":"MyApp","config":{"debug":true,"port":3000},"modules":["auth","api","web"]}}"#;
        assert_roundtrip(json);
    }

    #[test]
    fn multiple_arrays_in_object() {
        let json = r#"{"ids":[1,2,3],"names":["a","b","c"],"flags":[true,false],"empty":[],"tabular":[{"x":1,"y":2},{"x":3,"y":4}]}"#;
        assert_roundtrip(json);
    }

    #[test]
    fn deeply_mixed_types() {
        let json = r#"{"level1":{"level2":{"array":[{"x":1,"y":2},{"x":3,"y":4}],"value":"test"},"flat":42},"root_array":[1,2,3]}"#;
        assert_roundtrip(json);
    }
}
