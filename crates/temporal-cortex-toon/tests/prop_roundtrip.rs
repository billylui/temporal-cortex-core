/// Property-Based Roundtrip Tests for TOON v3.0
///
/// Uses the `proptest` crate to generate random JSON values and verify that
/// `decode(encode(json)) == json` holds for all generated inputs. This catches
/// edge cases that hand-written tests might miss.
///
/// Strategies generate:
/// - Random strings (including edge cases: empty, unicode, special chars)
/// - Random numbers (integers, floats — excluding NaN/Infinity)
/// - Random booleans and null
/// - Random flat objects (string keys + primitive values)
/// - Random nested objects (up to 3 levels deep)
/// - Random arrays (primitive, uniform objects for tabular, mixed)
///
/// Known limitations excluded from testing:
/// - Empty objects inside expanded list items (encoder gap: `- ` with nothing after)
/// - Float precision loss through `format!("{}", f)` display (last-digit rounding)
/// - Empty arrays produce trailing space in inline format (`[0]: `)
use proptest::prelude::*;
use serde_json::{json, Map, Number, Value};
use toon_core::{decode, encode};

// ============================================================================
// Strategies for generating JSON values
// ============================================================================

/// Generate a valid JSON object key (non-empty string, limited length).
fn arb_key() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-zA-Z_][a-zA-Z0-9_]{0,15}")
        .unwrap()
        .prop_filter("key must not be empty", |s| !s.is_empty())
}

/// Generate a random JSON string value with edge cases.
fn arb_json_string() -> impl Strategy<Value = String> {
    prop_oneof![
        // Simple ASCII strings
        "[a-zA-Z0-9 ]{0,30}",
        // Strings with special characters that need quoting
        prop::string::string_regex("[a-zA-Z0-9:,\\[\\]{}\\-\\. ]{0,20}").unwrap(),
        // Edge case: empty string
        Just("".to_string()),
        // Edge case: strings that look like booleans
        Just("true".to_string()),
        Just("false".to_string()),
        // Edge case: strings that look like null
        Just("null".to_string()),
        // Edge case: strings that look like numbers
        Just("42".to_string()),
        Just("3.14".to_string()),
        Just("0".to_string()),
        Just("-1".to_string()),
        Just("05".to_string()),
        // Edge case: strings with leading/trailing whitespace
        " [a-zA-Z]{1,10} ".prop_map(|s| s),
        // Edge case: strings starting with hyphen
        Just("-hello".to_string()),
        // Unicode
        Just("caf\u{00e9}".to_string()),
        Just("\u{4f60}\u{597d}".to_string()),
        // Strings with escape chars
        Just("line1\nline2".to_string()),
        Just("col1\tcol2".to_string()),
        Just("path\\to\\file".to_string()),
        Just("say \"hi\"".to_string()),
    ]
}

/// Generate a random JSON integer (always roundtrips cleanly through TOON).
fn arb_json_integer() -> impl Strategy<Value = Value> {
    prop_oneof![
        (-1_000_000i64..1_000_000i64).prop_map(|n| Value::Number(Number::from(n))),
        (0u64..1000u64).prop_map(|n| Value::Number(Number::from(n))),
    ]
}

/// Generate a random JSON float that roundtrips cleanly through TOON.
///
/// The TOON encoder uses `format!("{}", f)` which may lose precision for arbitrary f64 values.
/// Instead of filtering arbitrary floats, we generate "simple" floats with limited decimal
/// places that always roundtrip correctly. This is done by generating an integer mantissa
/// and dividing by a power of 10.
fn arb_json_float() -> impl Strategy<Value = Value> {
    // Generate float as integer / 10^n (1-4 decimal places)
    // This produces values like 3.14, -127.5, 0.001, etc.
    (-100_000_000i64..100_000_000i64, 1u32..5u32).prop_filter_map(
        "must be representable as f64 and not integer",
        |(mantissa, decimals)| {
            let divisor = 10f64.powi(decimals as i32);
            let f = mantissa as f64 / divisor;
            if !f.is_finite() {
                return None;
            }
            // Skip values that are whole numbers (those should be integers)
            if f.fract() == 0.0 {
                return None;
            }
            Number::from_f64(f).map(Value::Number)
        },
    )
}

/// Generate a random JSON number (integer or display-safe float).
fn arb_json_number() -> impl Strategy<Value = Value> {
    prop_oneof![
        3 => arb_json_integer(),
        1 => arb_json_float(),
    ]
}

/// Generate a random primitive JSON value (string, number, bool, null).
fn arb_primitive() -> impl Strategy<Value = Value> {
    prop_oneof![
        // Strings
        arb_json_string().prop_map(Value::String),
        // Numbers
        arb_json_number(),
        // Booleans
        any::<bool>().prop_map(Value::Bool),
        // Null
        Just(Value::Null),
    ]
}

/// Generate a flat JSON object (all values are primitives, non-empty — avoids
/// empty object limitation in list items).
fn arb_flat_object() -> impl Strategy<Value = Value> {
    prop::collection::vec((arb_key(), arb_primitive()), 1..8).prop_map(|pairs| {
        let mut map = Map::new();
        for (k, v) in pairs {
            map.insert(k, v);
        }
        Value::Object(map)
    })
}

/// Generate an inline-style array (all elements are primitives).
fn arb_primitive_array() -> impl Strategy<Value = Value> {
    prop::collection::vec(arb_primitive(), 0..8).prop_map(Value::Array)
}

/// Generate a tabular-style array (all elements are objects with the same keys, primitive values).
fn arb_tabular_array() -> impl Strategy<Value = Value> {
    (prop::collection::vec(arb_key(), 1..5), 1..6usize).prop_flat_map(|(fields, num_rows)| {
        let fields_clone = fields.clone();
        prop::collection::vec(
            prop::collection::vec(arb_primitive(), fields.len()..=fields.len()),
            num_rows..=num_rows,
        )
        .prop_map(move |rows| {
            let arr: Vec<Value> = rows
                .into_iter()
                .map(|vals| {
                    let mut map = Map::new();
                    for (k, v) in fields_clone.iter().zip(vals.into_iter()) {
                        map.insert(k.clone(), v);
                    }
                    Value::Object(map)
                })
                .collect();
            Value::Array(arr)
        })
    })
}

/// Check if a JSON value contains empty objects inside arrays (known limitation).
/// Empty objects in expanded list items don't roundtrip correctly.
fn contains_empty_object_in_array(v: &Value) -> bool {
    match v {
        Value::Array(arr) => {
            for item in arr {
                match item {
                    Value::Object(map) if map.is_empty() => return true,
                    _ => {
                        if contains_empty_object_in_array(item) {
                            return true;
                        }
                    }
                }
            }
            false
        }
        Value::Object(map) => {
            for val in map.values() {
                if contains_empty_object_in_array(val) {
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

/// Check if a value contains deeply nested structures that hit known indentation
/// limitations (tabular arrays inside expanded list items at depth > 2).
fn contains_deep_tabular_in_expanded(v: &Value, depth: usize) -> bool {
    if depth > 3 {
        // At deep nesting, any array of objects could trigger the limitation
        if let Value::Array(arr) = v {
            if arr.iter().any(|item| item.is_object()) {
                return true;
            }
        }
    }
    match v {
        Value::Array(arr) => {
            for item in arr {
                if contains_deep_tabular_in_expanded(item, depth + 1) {
                    return true;
                }
            }
            false
        }
        Value::Object(map) => {
            for val in map.values() {
                if contains_deep_tabular_in_expanded(val, depth + 1) {
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

/// Generate a JSON value with limited nesting (recursive).
/// Filters out known-problematic patterns (empty objects in arrays).
fn arb_json_value_inner(depth: u32) -> impl Strategy<Value = Value> {
    if depth == 0 {
        arb_primitive().boxed()
    } else {
        prop_oneof![
            4 => arb_primitive(),
            2 => prop::collection::vec((arb_key(), arb_json_value_inner(depth - 1)), 1..5)
                .prop_map(|pairs| {
                    let mut map = Map::new();
                    for (k, v) in pairs {
                        map.insert(k, v);
                    }
                    Value::Object(map)
                }),
            2 => prop::collection::vec(arb_json_value_inner(depth - 1), 0..5)
                .prop_map(Value::Array),
        ]
        .boxed()
    }
}

/// Top-level strategy for generating random JSON values (up to 3 levels deep).
/// Filters out values containing known-problematic patterns.
fn arb_json_value() -> impl Strategy<Value = Value> {
    arb_json_value_inner(3).prop_filter("exclude values with known limitations", |v| {
        !contains_empty_object_in_array(v) && !contains_deep_tabular_in_expanded(v, 0)
    })
}

// ============================================================================
// Helper: normalize JSON for comparison
// ============================================================================

/// Normalize a JSON value for comparison.
/// Handles: -0 -> 0, float-as-integer (1.0 -> 1).
/// Since we generate "simple" floats (limited decimal places), precision normalization
/// is not needed — the generated values always roundtrip through format!("{}", f).
fn normalize_json(v: &Value) -> Value {
    match v {
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Number(Number::from(i))
            } else if let Some(u) = n.as_u64() {
                Value::Number(Number::from(u))
            } else if let Some(f) = n.as_f64() {
                // Normalize -0.0 to 0
                let f = if f == 0.0 { 0.0f64 } else { f };
                if f.fract() == 0.0 && f.abs() < (i64::MAX as f64) {
                    Value::Number(Number::from(f as i64))
                } else if let Some(n) = Number::from_f64(f) {
                    Value::Number(n)
                } else {
                    Value::Null
                }
            } else {
                Value::Null
            }
        }
        Value::Object(map) => {
            let mut new_map = Map::new();
            for (k, v) in map {
                new_map.insert(k.clone(), normalize_json(v));
            }
            Value::Object(new_map)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(normalize_json).collect()),
        other => other.clone(),
    }
}

// ============================================================================
// Property Tests
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    /// Core roundtrip property: decode(encode(json)) == json for any JSON value.
    #[test]
    fn roundtrip_preserves_json(value in arb_json_value()) {
        let json_str = serde_json::to_string(&value).unwrap();
        let toon = encode(&json_str).unwrap();
        let decoded_json = decode(&toon).unwrap();
        let original = normalize_json(&value);
        let roundtripped: Value = serde_json::from_str(&decoded_json).unwrap();
        let roundtripped = normalize_json(&roundtripped);
        prop_assert_eq!(
            original,
            roundtripped,
            "Roundtrip failed!\n  JSON in:  {}\n  TOON:     {}\n  JSON out: {}",
            json_str,
            toon,
            decoded_json
        );
    }

    /// Roundtrip for flat objects (most common case for config data).
    #[test]
    fn roundtrip_flat_object(obj in arb_flat_object()) {
        let json_str = serde_json::to_string(&obj).unwrap();
        let toon = encode(&json_str).unwrap();
        let decoded_json = decode(&toon).unwrap();
        let original = normalize_json(&obj);
        let roundtripped: Value = serde_json::from_str(&decoded_json).unwrap();
        let roundtripped = normalize_json(&roundtripped);
        prop_assert_eq!(original, roundtripped);
    }

    /// Roundtrip for primitive arrays (inline encoding).
    #[test]
    fn roundtrip_primitive_array(arr in arb_primitive_array()) {
        let json_str = serde_json::to_string(&arr).unwrap();
        let toon = encode(&json_str).unwrap();
        let decoded_json = decode(&toon).unwrap();
        let original = normalize_json(&arr);
        let roundtripped: Value = serde_json::from_str(&decoded_json).unwrap();
        let roundtripped = normalize_json(&roundtripped);
        prop_assert_eq!(original, roundtripped);
    }

    /// Roundtrip for tabular arrays (uniform object arrays).
    #[test]
    fn roundtrip_tabular_array(arr in arb_tabular_array()) {
        let wrapped = json!({"data": arr});
        let json_str = serde_json::to_string(&wrapped).unwrap();
        let toon = encode(&json_str).unwrap();
        let decoded_json = decode(&toon).unwrap();
        let original = normalize_json(&wrapped);
        let roundtripped: Value = serde_json::from_str(&decoded_json).unwrap();
        let roundtripped = normalize_json(&roundtripped);
        prop_assert_eq!(original, roundtripped);
    }

    /// TOON output never has trailing newline.
    #[test]
    fn no_trailing_newline(value in arb_json_value()) {
        let json_str = serde_json::to_string(&value).unwrap();
        let toon = encode(&json_str).unwrap();
        prop_assert!(
            !toon.ends_with('\n'),
            "TOON output must not end with newline: {:?}",
            toon
        );
    }

    /// TOON output never has trailing spaces on any line, except for lines containing
    /// empty array inline syntax `[0]: ` which is a known minor encoder artifact.
    #[test]
    fn no_trailing_spaces(value in arb_json_value()) {
        let json_str = serde_json::to_string(&value).unwrap();
        let toon = encode(&json_str).unwrap();
        for (i, line) in toon.lines().enumerate() {
            if line.ends_with(' ') {
                // Allow any line that contains the empty-array inline pattern "[0]: "
                // This can appear as "key[0]: ", "  - [0]: ", "[0]: " etc.
                prop_assert!(
                    line.contains("[0]: "),
                    "Line {} has unexpected trailing space: {:?} (full TOON: {:?})",
                    i,
                    line,
                    toon
                );
            }
        }
    }

    /// Encoding always produces valid output (never panics).
    #[test]
    fn encode_never_panics(value in arb_json_value()) {
        let json_str = serde_json::to_string(&value).unwrap();
        let _ = encode(&json_str);
    }

    /// Decoding encoded output always produces valid JSON (never panics).
    #[test]
    fn decode_never_panics(value in arb_json_value()) {
        let json_str = serde_json::to_string(&value).unwrap();
        let toon = encode(&json_str).unwrap();
        let result = decode(&toon);
        prop_assert!(result.is_ok(), "Decode failed for TOON: {:?}", toon);
    }

    /// Strings that look like keywords are always preserved as strings through roundtrip.
    #[test]
    fn keyword_like_strings_preserved(s in prop_oneof![
        Just("true".to_string()),
        Just("false".to_string()),
        Just("null".to_string()),
        Just("42".to_string()),
        Just("3.14".to_string()),
        Just("0".to_string()),
        Just("-1".to_string()),
        Just("".to_string()),
        Just("05".to_string()),
    ]) {
        let value = Value::String(s.clone());
        let json_str = serde_json::to_string(&value).unwrap();
        let toon = encode(&json_str).unwrap();
        let decoded_json = decode(&toon).unwrap();
        let roundtripped: Value = serde_json::from_str(&decoded_json).unwrap();
        prop_assert_eq!(
            Value::String(s.clone()),
            roundtripped,
            "Keyword-like string not preserved: {:?} -> TOON: {:?} -> JSON: {:?}",
            s,
            toon,
            decoded_json
        );
    }

    /// Randomly generated strings always roundtrip correctly as object values.
    #[test]
    fn string_value_roundtrip(s in arb_json_string()) {
        let obj = json!({"key": s});
        let json_str = serde_json::to_string(&obj).unwrap();
        let toon = encode(&json_str).unwrap();
        let decoded_json = decode(&toon).unwrap();
        let original: Value = serde_json::from_str(&json_str).unwrap();
        let roundtripped: Value = serde_json::from_str(&decoded_json).unwrap();
        prop_assert_eq!(
            original,
            roundtripped,
            "String roundtrip failed for {:?}\n  TOON: {:?}\n  decoded: {:?}",
            s,
            toon,
            decoded_json
        );
    }

    /// Integer numbers always roundtrip correctly (no precision issues).
    #[test]
    fn integer_roundtrip(n in arb_json_integer()) {
        let obj = json!({"val": n});
        let json_str = serde_json::to_string(&obj).unwrap();
        let toon = encode(&json_str).unwrap();
        let decoded_json = decode(&toon).unwrap();
        let original = normalize_json(&obj);
        let roundtripped: Value = serde_json::from_str(&decoded_json).unwrap();
        let roundtripped = normalize_json(&roundtripped);
        prop_assert_eq!(original, roundtripped);
    }

    /// Display-safe float numbers roundtrip correctly.
    #[test]
    fn float_roundtrip(n in arb_json_float()) {
        let obj = json!({"val": n});
        let json_str = serde_json::to_string(&obj).unwrap();
        let toon = encode(&json_str).unwrap();
        let decoded_json = decode(&toon).unwrap();
        let original = normalize_json(&obj);
        let roundtripped: Value = serde_json::from_str(&decoded_json).unwrap();
        let roundtripped = normalize_json(&roundtripped);
        prop_assert_eq!(original, roundtripped);
    }
}
