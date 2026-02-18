//! TOON v3.0 Encoder — converts JSON into Token-Oriented Object Notation.
//!
//! TOON is a compact, human-readable format designed to minimize token usage when
//! feeding structured data to LLMs. The encoder implements the full TOON v3.0 spec
//! (2025-11-24), including:
//!
//! - **Key folding**: nested objects expressed via indentation, no braces/brackets
//! - **Inline arrays**: primitive arrays as `key[N]: v1,v2,v3`
//! - **Tabular arrays**: uniform object arrays as `key[N]{f1,f2}:\n  v1,v2\n  v3,v4`
//! - **Expanded lists**: mixed/complex arrays as `key[N]:\n  - item1\n  - item2`
//! - **Context-dependent quoting**: strings only quoted when ambiguous (per delimiter scope)
//! - **Number normalization**: no exponents, no trailing zeros, -0 → 0
//!
//! # Example
//! ```
//! use toon_core::encode;
//! let json = r#"{"name":"Alice","age":30,"tags":["rust","wasm"]}"#;
//! let toon = encode(json).unwrap();
//! // name: Alice
//! // age: 30
//! // tags[2]: rust,wasm
//! ```

use crate::error::Result;
use serde_json::Value;

/// Encode a JSON string into TOON v3.0 format.
///
/// Parses the input as JSON, then walks the value tree to produce a compact TOON
/// representation. Returns an error if the input is not valid JSON.
pub fn encode(json: &str) -> Result<String> {
    let value: Value = serde_json::from_str(json)?;
    let mut out = String::new();
    encode_root(&value, &mut out);
    Ok(out)
}

/// Top-level dispatch: objects emit fields, arrays emit root array syntax,
/// primitives emit a bare value.
fn encode_root(value: &Value, out: &mut String) {
    match value {
        Value::Object(map) => {
            encode_object_fields(map, 0, out);
        }
        Value::Array(arr) => {
            encode_root_array(arr, out);
        }
        _ => {
            encode_primitive_value(value, QuoteContext::Document, out);
        }
    }
}

/// Encode a root-level array. Primitive arrays use inline syntax `[N]: v1,v2`;
/// mixed/complex arrays use expanded list syntax `[N]:\n  - item`.
fn encode_root_array(arr: &[Value], out: &mut String) {
    let len = arr.len();
    if all_primitives(arr) {
        out.push_str(&format!("[{}]: ", len));
        encode_inline_values(arr, out);
    } else {
        out.push_str(&format!("[{}]:", len));
        encode_list_items(arr, 0, out);
    }
}

/// Emit all key-value pairs of an object at the given indentation depth.
/// Each field appears on its own line; values are dispatched by type.
///
/// Relies on `serde_json::Map` with `preserve_order` feature to maintain
/// the original JSON insertion order (IndexMap, not BTreeMap).
fn encode_object_fields(map: &serde_json::Map<String, Value>, depth: usize, out: &mut String) {
    let indent = make_indent(depth);
    let mut first = true;
    for (key, value) in map {
        if !first {
            out.push('\n');
        }
        first = false;
        out.push_str(&indent);
        out.push_str(&encode_key(key));
        encode_field_value(key, value, depth, out);
    }
}

/// Dispatch a field's value to the appropriate TOON encoding:
/// - Empty objects → `key:`
/// - Non-empty objects → `key:\n  child_key: child_val`
/// - Arrays → delegated to `encode_array_field` (inline/tabular/expanded)
/// - Primitives → `key: value`
fn encode_field_value(_key: &str, value: &Value, depth: usize, out: &mut String) {
    match value {
        Value::Object(map) if map.is_empty() => {
            out.push(':');
        }
        Value::Object(map) => {
            out.push(':');
            out.push('\n');
            encode_object_fields(map, depth + 1, out);
        }
        Value::Array(arr) => {
            encode_array_field(arr, depth, out);
        }
        _ => {
            out.push_str(": ");
            encode_primitive_value(value, QuoteContext::Document, out);
        }
    }
}

/// Encode an array field value, selecting the most compact TOON representation:
///
/// 1. **Empty**: `key[0]:`
/// 2. **Tabular**: all elements are objects with identical primitive-only keys →
///    `key[N]{f1,f2}:\n  v1,v2\n  v3,v4`
/// 3. **Inline**: all elements are primitives → `key[N]: v1,v2,v3`
/// 4. **Expanded list**: mixed content → `key[N]:\n  - item1\n  - item2`
fn encode_array_field(arr: &[Value], depth: usize, out: &mut String) {
    let len = arr.len();

    if arr.is_empty() {
        out.push_str(&format!("[{}]:", len));
        return;
    }

    // Tabular: uniform object arrays (greatest compression for repetitive data)
    if let Some(fields) = detect_tabular(arr) {
        out.push_str(&format!("[{}]{{{}}}:", len, fields.join(",")));
        encode_tabular_rows(arr, &fields, depth, out);
        return;
    }

    // Inline: all-primitive arrays on a single line
    if all_primitives(arr) {
        out.push_str(&format!("[{}]: ", len));
        encode_inline_values(arr, out);
        return;
    }

    // Expanded: complex/mixed arrays with "- " list markers
    out.push_str(&format!("[{}]:", len));
    encode_list_items(arr, depth, out);
}

/// Emit comma-separated primitive values on a single line: `v1,v2,v3`
/// Quoting uses `InlineArray` context (comma is the active delimiter, not colon).
fn encode_inline_values(arr: &[Value], out: &mut String) {
    for (i, val) in arr.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        encode_primitive_value(val, QuoteContext::InlineArray, out);
    }
}

/// Emit tabular rows: each object's values as a comma-separated line, no keys repeated.
/// Quoting uses `TabularCell` context (comma triggers quoting, not colon).
fn encode_tabular_rows(arr: &[Value], fields: &[String], depth: usize, out: &mut String) {
    let row_indent = make_indent(depth + 1);
    for obj_val in arr {
        out.push('\n');
        out.push_str(&row_indent);
        if let Value::Object(map) = obj_val {
            for (i, field) in fields.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                if let Some(val) = map.get(field) {
                    encode_primitive_value(val, QuoteContext::TabularCell, out);
                }
            }
        }
    }
}

/// Emit expanded list items with "- " markers. Each item can be:
/// - A primitive value: `- hello`
/// - An object: `- key1: val1\n    key2: val2` (first field on hyphen line)
/// - A nested array: `- [N]: v1,v2`
fn encode_list_items(arr: &[Value], depth: usize, out: &mut String) {
    let item_indent = make_indent(depth + 1);
    for item in arr {
        out.push('\n');
        out.push_str(&item_indent);
        out.push_str("- ");
        match item {
            Value::Object(map) => {
                // First field on the hyphen line
                let mut first = true;
                for (key, value) in map {
                    if first {
                        first = false;
                        out.push_str(&encode_key(key));
                        encode_list_item_field_value(value, depth + 1, out);
                    } else {
                        out.push('\n');
                        // Sibling fields at same depth as "- " content
                        out.push_str(&make_indent(depth + 1));
                        out.push_str("  ");
                        out.push_str(&encode_key(key));
                        encode_list_item_field_value(value, depth + 1, out);
                    }
                }
            }
            Value::Array(inner_arr) => {
                // Nested array as list item
                let len = inner_arr.len();
                if all_primitives(inner_arr) {
                    out.push_str(&format!("[{}]: ", len));
                    encode_inline_values(inner_arr, out);
                } else {
                    out.push_str(&format!("[{}]:", len));
                    encode_list_items(inner_arr, depth + 1, out);
                }
            }
            _ => {
                encode_primitive_value(item, QuoteContext::Document, out);
            }
        }
    }
}

/// Encode a field value within a list item object. Differs from `encode_field_value`
/// because nested objects inside list items use an extra indent level to account
/// for the "- " prefix offset.
fn encode_list_item_field_value(value: &Value, depth: usize, out: &mut String) {
    match value {
        Value::Object(map) if map.is_empty() => {
            out.push(':');
        }
        Value::Object(map) => {
            out.push(':');
            out.push('\n');
            // Nested object inside a list item: depth + 1 extra for the "- " offset
            let nested_indent = make_indent(depth + 2);
            let mut first = true;
            for (key, val) in map {
                if !first {
                    out.push('\n');
                }
                first = false;
                out.push_str(&nested_indent);
                out.push_str(&encode_key(key));
                encode_field_value(key, val, depth + 2, out);
            }
        }
        Value::Array(arr) => {
            encode_array_field(arr, depth, out);
        }
        _ => {
            out.push_str(": ");
            encode_primitive_value(value, QuoteContext::Document, out);
        }
    }
}

/// Context for quoting decisions per TOON v3.0 delimiter scoping rules.
#[derive(Clone, Copy, PartialEq)]
enum QuoteContext {
    /// Object field value or bare root primitive — colon triggers quoting
    Document,
    /// Inline primitive array value — comma (active delimiter) triggers quoting
    InlineArray,
    /// Tabular row cell — comma (active delimiter) triggers quoting, NOT colon
    TabularCell,
}

/// Emit a primitive JSON value (null, bool, number, string) in TOON format.
/// String quoting depends on the `QuoteContext` — different delimiters are
/// "active" in different positions (see TOON v3.0 spec, delimiter scoping).
fn encode_primitive_value(value: &Value, ctx: QuoteContext, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&format_number(n)),
        Value::String(s) => encode_string_value(s, ctx, out),
        _ => out.push_str("null"), // arrays/objects in primitive context
    }
}

/// Format a JSON number per TOON v3.0 rules:
/// - No scientific notation (exponents)
/// - No leading zeros (except 0.x)
/// - No trailing fractional zeros (3.10 → 3.1)
/// - Negative zero normalizes to 0
fn format_number(n: &serde_json::Number) -> String {
    if let Some(i) = n.as_i64() {
        return i.to_string();
    }
    if let Some(u) = n.as_u64() {
        return u.to_string();
    }
    if let Some(f) = n.as_f64() {
        if f.is_nan() || f.is_infinite() {
            return "null".to_string();
        }
        // Normalize -0 to 0
        let f = if f == 0.0 { 0.0 } else { f };
        // Check if it's a whole number
        if f.fract() == 0.0 && f.abs() < (i64::MAX as f64) {
            return (f as i64).to_string();
        }
        // Format without trailing zeros
        let s = format!("{}", f);
        // Remove trailing zeros after decimal point
        if s.contains('.') {
            let trimmed = s.trim_end_matches('0');
            let trimmed = trimmed.trim_end_matches('.');
            trimmed.to_string()
        } else {
            s
        }
    } else {
        "null".to_string()
    }
}

/// Emit a string value, quoting and escaping only when necessary.
/// Unquoted strings save 2 tokens (the quotes) per value — significant at scale.
fn encode_string_value(s: &str, ctx: QuoteContext, out: &mut String) {
    if needs_quoting(s, ctx) {
        out.push('"');
        for ch in s.chars() {
            match ch {
                '\\' => out.push_str("\\\\"),
                '"' => out.push_str("\\\""),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                _ => out.push(ch),
            }
        }
        out.push('"');
    } else {
        out.push_str(s);
    }
}

/// Determine if a string value must be quoted to preserve TOON roundtrip fidelity.
///
/// A string MUST be quoted if it:
/// - Is empty
/// - Has leading/trailing whitespace
/// - Looks like a boolean (`true`/`false`) or `null`
/// - Looks numeric (would be decoded as a number instead of string)
/// - Contains backslash, double quote, brackets, braces, or control chars
/// - Starts with `-` (ambiguous with list item marker)
/// - Contains the ACTIVE delimiter for the current context:
///   - Document context: colon (`:`)
///   - InlineArray/TabularCell context: comma (`,`)
fn needs_quoting(s: &str, ctx: QuoteContext) -> bool {
    // Empty string
    if s.is_empty() {
        return true;
    }
    // Leading or trailing whitespace
    if s != s.trim() {
        return true;
    }
    // Looks like bool or null
    if s == "true" || s == "false" || s == "null" {
        return true;
    }
    // Looks like a number (including leading-zero forms like "05")
    if looks_numeric(s) {
        return true;
    }
    // Contains backslash or double quote
    if s.contains('\\') || s.contains('"') {
        return true;
    }
    // Contains brackets or braces
    if s.contains('[') || s.contains(']') || s.contains('{') || s.contains('}') {
        return true;
    }
    // Contains control characters
    if s.contains('\n') || s.contains('\r') || s.contains('\t') {
        return true;
    }
    // Starts with hyphen (could be confused with list item marker "- ")
    if s.starts_with('-') {
        return true;
    }
    // Context-dependent delimiter quoting
    match ctx {
        QuoteContext::Document => {
            // Colon triggers quoting in document context
            if s.contains(':') {
                return true;
            }
        }
        QuoteContext::InlineArray | QuoteContext::TabularCell => {
            // Active delimiter (comma by default) triggers quoting
            if s.contains(',') {
                return true;
            }
        }
    }
    false
}

/// Check if a string looks like a number (and thus must be quoted to preserve type info).
/// Matches integers, floats, and leading-zero forms like "05" or "0001".
fn looks_numeric(s: &str) -> bool {
    // Matches numeric patterns: integers, floats, leading-zero forms
    if s.is_empty() {
        return false;
    }
    let bytes = s.as_bytes();
    let start = if bytes[0] == b'-' { 1 } else { 0 };
    if start >= bytes.len() {
        return false;
    }
    // All remaining must be digits, optionally with one dot and optional exponent
    let rest = &s[start..];
    if rest.is_empty() {
        return false;
    }
    // Check for leading-zero forms like "05", "0001"
    if rest.len() > 1 && rest.starts_with('0') && rest.as_bytes()[1] != b'.' {
        return true; // "05", "00" etc. are numeric-like
    }
    // Try to parse as a number pattern
    let mut has_dot = false;
    let mut has_e = false;
    for (i, &b) in rest.as_bytes().iter().enumerate() {
        match b {
            b'0'..=b'9' => {}
            b'.' if !has_dot && !has_e => has_dot = true,
            b'e' | b'E' if !has_e && i > 0 => has_e = true,
            b'+' | b'-' if has_e => {}
            _ => return false,
        }
    }
    // Must have at least one digit
    rest.as_bytes().iter().any(|b| b.is_ascii_digit())
}

/// Encode an object key. Keys matching `^[A-Za-z_][A-Za-z0-9_.]*$` are emitted
/// unquoted; all others are quoted with escape sequences.
fn encode_key(key: &str) -> String {
    if is_valid_unquoted_key(key) {
        key.to_string()
    } else {
        let mut out = String::with_capacity(key.len() + 2);
        out.push('"');
        for ch in key.chars() {
            match ch {
                '\\' => out.push_str("\\\\"),
                '"' => out.push_str("\\\""),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                _ => out.push(ch),
            }
        }
        out.push('"');
        out
    }
}

/// Test if a key can be emitted unquoted per TOON v3.0: `^[A-Za-z_][A-Za-z0-9_.]*$`
fn is_valid_unquoted_key(key: &str) -> bool {
    // Must match: ^[A-Za-z_][A-Za-z0-9_.]*$
    if key.is_empty() {
        return false;
    }
    let mut chars = key.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
}

/// Detect if an array is tabular: all elements are objects with identical key sets,
/// all values are primitives (no nested arrays/objects).
fn detect_tabular(arr: &[Value]) -> Option<Vec<String>> {
    if arr.is_empty() {
        return None;
    }
    // All must be objects
    let first = arr[0].as_object()?;
    let fields: Vec<String> = first.keys().cloned().collect();
    if fields.is_empty() {
        return None;
    }
    // All values in first object must be primitive
    for val in first.values() {
        if val.is_object() || val.is_array() {
            return None;
        }
    }
    // All subsequent objects must have the same keys with primitive values
    for item in &arr[1..] {
        let obj = item.as_object()?;
        if obj.len() != fields.len() {
            return None;
        }
        for field in &fields {
            let val = obj.get(field)?;
            if val.is_object() || val.is_array() {
                return None;
            }
        }
    }
    Some(fields)
}

/// Check if all array elements are primitives (not objects or arrays).
fn all_primitives(arr: &[Value]) -> bool {
    arr.iter().all(|v| !v.is_object() && !v.is_array())
}

/// Generate a 2-space-per-level indentation string.
fn make_indent(depth: usize) -> String {
    "  ".repeat(depth)
}
