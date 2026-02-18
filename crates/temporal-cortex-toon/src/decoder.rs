//! TOON v3.0 Decoder — converts TOON back into JSON.
//!
//! The decoder parses indentation-based TOON structure back into a `serde_json::Value`
//! tree. It handles all TOON v3.0 constructs:
//!
//! - Flat and nested objects (indentation-based)
//! - Inline primitive arrays (`key[N]: v1,v2`)
//! - Tabular arrays (`key[N]{f1,f2}:\n  v1,v2`)
//! - Expanded lists (`key[N]:\n  - item`)
//! - Quoted/unquoted keys and values with escape sequences
//! - Type inference: unquoted `true`/`false` → bool, `null` → null, numbers → number
//!
//! # Key design decisions
//!
//! - **Line-index tracking**: `parse_key_value_into_map` returns the next line index
//!   so callers can correctly skip past array bodies in nested structures.
//! - **`skip_array_body` vs `skip_nested_lines`**: Array bodies containing "- " list
//!   items need special handling to avoid skipping sibling fields at the same indent.
//! - **Auto-detected indent**: `parse_array_body` finds the first "- " line's indent
//!   rather than assuming `base_indent + 2`, supporting flexible nesting depths.

use crate::error::{Result, ToonError};
use serde_json::{Map, Value};

/// Decode a TOON string back into JSON format.
///
/// Takes a valid TOON string and returns the compact JSON representation.
/// The output is minified (no pretty-printing) — use `serde_json::to_string_pretty`
/// on the result if human-readable JSON is needed.
pub fn decode(toon: &str) -> Result<String> {
    let value = parse_toon(toon)?;
    Ok(serde_json::to_string(&value)?)
}

/// Main entry point: classify the TOON input as root array, root primitive, or object.
fn parse_toon(toon: &str) -> Result<Value> {
    let toon = toon.trim_end_matches('\n');

    if toon.is_empty() {
        return Ok(Value::Object(Map::new()));
    }

    // Check for root array: starts with [N]:
    if toon.starts_with('[') {
        if let Some(val) = try_parse_root_array(toon)? {
            return Ok(val);
        }
    }

    // Check for root primitive (single line, no colon structure)
    let lines: Vec<&str> = toon.lines().collect();
    if lines.len() == 1 && !line_has_key_colon(lines[0]) {
        return parse_primitive_value(lines[0].trim());
    }

    // Object: key-value pairs
    parse_object_from_lines(&lines, 0, 0, lines.len())
}

/// Try parsing as root array: [N]: ... or [N]:\n...
fn try_parse_root_array(toon: &str) -> Result<Option<Value>> {
    let lines: Vec<&str> = toon.lines().collect();
    if lines.is_empty() {
        return Ok(None);
    }
    let first_line = lines[0];

    // Match [N]{fields}: or [N]: or [N]:
    if let Some(header) = parse_array_header(first_line) {
        let arr = parse_array_body(&header, &lines, 0, 0)?;
        return Ok(Some(arr));
    }
    Ok(None)
}

/// Check if a line has a key: pattern (not just a primitive that happens to contain ':')
fn line_has_key_colon(line: &str) -> bool {
    let trimmed = line.trim();
    // If it starts with a quote, it could be a quoted key
    if trimmed.starts_with('"') {
        // Find the closing quote (handling escapes)
        if let Some(end) = find_closing_quote(trimmed, 1) {
            // After closing quote, should be ':'
            return end + 1 < trimmed.len() && trimmed.as_bytes()[end + 1] == b':';
        }
        return false;
    }
    // If it starts with '[', could be a root array header
    if trimmed.starts_with('[') {
        return false;
    }
    // Check for unquoted key: look for first ':' not inside quotes
    // Simple heuristic: if there's a colon, and the part before it looks like a key
    if let Some(colon_pos) = trimmed.find(':') {
        let before = &trimmed[..colon_pos];
        // Key should not contain spaces (unquoted keys are [A-Za-z_][A-Za-z0-9_.]*)
        !before.contains(' ') && !before.is_empty()
    } else {
        false
    }
}

/// Parsed metadata from an array header line like `key[3]{a,b}: ` or `key[2]: v1,v2`.
///
/// - `len`: declared element count (used for validation, not currently enforced)
/// - `fields`: tabular column names if present (`{f1,f2}` syntax)
/// - `inline_values`: the raw value string if inline (`[N]: v1,v2` — text after `: `)
struct ArrayHeader {
    len: usize,
    fields: Option<Vec<String>>,
    inline_values: Option<String>,
}

/// Parse array header from a line like `[N]: v1,v2` or `[N]{f1,f2}:` or `[N]:`
fn parse_array_header(line: &str) -> Option<ArrayHeader> {
    let trimmed = line.trim();
    let bracket_start = trimmed.find('[')?;
    let bracket_end = trimmed[bracket_start..].find(']')? + bracket_start;
    let len_str = &trimmed[bracket_start + 1..bracket_end];
    let len: usize = len_str.parse().ok()?;

    let after_bracket = &trimmed[bracket_end + 1..];

    // Check for tabular: {f1,f2}:
    if after_bracket.starts_with('{') {
        let brace_end = after_bracket.find('}')?;
        let fields_str = &after_bracket[1..brace_end];
        let fields: Vec<String> = fields_str.split(',').map(|s| s.to_string()).collect();
        let after_brace = &after_bracket[brace_end + 1..];
        if after_brace.starts_with(':') {
            return Some(ArrayHeader {
                len,
                fields: Some(fields),
                inline_values: None,
            });
        }
        return None;
    }

    // Check for inline: `: v1,v2` (space after colon with values on same line)
    if let Some(values) = after_bracket.strip_prefix(": ") {
        return Some(ArrayHeader {
            len,
            fields: None,
            inline_values: Some(values.to_string()),
        });
    }

    // Expanded/empty: `:`
    if after_bracket.starts_with(':') {
        return Some(ArrayHeader {
            len,
            fields: None,
            inline_values: None,
        });
    }

    None
}

/// Parse the body of an array given its header and surrounding lines.
///
/// Dispatches to inline parsing, tabular row parsing, or expanded list parsing
/// based on the header type. For expanded lists, auto-detects the indent of the
/// first "- " marker rather than assuming a fixed offset.
fn parse_array_body(
    header: &ArrayHeader,
    lines: &[&str],
    line_idx: usize,
    base_indent: usize,
) -> Result<Value> {
    // Empty array
    if header.len == 0 {
        return Ok(Value::Array(vec![]));
    }

    // Inline values
    if let Some(ref inline) = header.inline_values {
        let values = parse_inline_values(inline)?;
        return Ok(Value::Array(values));
    }

    // Tabular
    if let Some(ref fields) = header.fields {
        let mut rows = Vec::new();
        for (i, line) in lines.iter().enumerate().skip(line_idx + 1) {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            // Check indent — tabular rows should be at base_indent + 2
            let indent = count_indent(line);
            if indent <= base_indent && i > line_idx + 1 {
                break;
            }
            let obj = parse_tabular_row(trimmed, fields)?;
            rows.push(obj);
        }
        return Ok(Value::Array(rows));
    }

    // Expanded list (- items)
    // Auto-detect the indent of the first "- " line
    let mut detected_indent = base_indent + 2;
    for line in &lines[line_idx + 1..] {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with("- ") {
            detected_indent = count_indent(line);
            break;
        }
        break;
    }
    parse_list_items(lines, line_idx + 1, detected_indent)
}

/// Parse comma-separated inline values like `1,Alice,true`.
/// Handles quoted values with escape sequences (e.g., `"hello, world",42,true`).
fn parse_inline_values(s: &str) -> Result<Vec<Value>> {
    let mut values = Vec::new();
    let mut i = 0;
    let bytes = s.as_bytes();

    while i < bytes.len() {
        if bytes[i] == b'"' {
            // Quoted value
            let end = find_closing_quote(s, i + 1).ok_or_else(|| ToonError::ToonParse {
                line: 0,
                message: "Unterminated quoted string in inline array".to_string(),
            })?;
            let inner = &s[i + 1..end];
            let unescaped = unescape_string(inner);
            values.push(Value::String(unescaped));
            i = end + 1;
            // Skip comma
            if i < bytes.len() && bytes[i] == b',' {
                i += 1;
            }
        } else {
            // Unquoted value — find next comma
            let end = s[i..].find(',').map(|p| p + i).unwrap_or(s.len());
            let token = &s[i..end];
            values.push(parse_primitive_token(token));
            i = end;
            if i < bytes.len() && bytes[i] == b',' {
                i += 1;
            }
        }
    }

    Ok(values)
}

/// Parse a tabular row: comma-separated values mapped to field names
fn parse_tabular_row(row: &str, fields: &[String]) -> Result<Value> {
    let values = parse_inline_values(row)?;
    let mut map = Map::new();
    for (i, field) in fields.iter().enumerate() {
        let val = values.get(i).cloned().unwrap_or(Value::Null);
        map.insert(field.clone(), val);
    }
    Ok(Value::Object(map))
}

/// Parse expanded list items starting from a given line index.
///
/// `item_indent` is the character offset where "- " markers appear. Items at this
/// indent are collected; lines deeper than `item_indent` belong to the current item;
/// lines shallower terminate the list. Lines at `item_indent` without "- " also
/// terminate (they're sibling fields, not list items).
fn parse_list_items(lines: &[&str], start_line: usize, item_indent: usize) -> Result<Value> {
    let mut items = Vec::new();
    let mut i = start_line;

    while i < lines.len() {
        let line = lines[i];
        let indent = count_indent(line);
        let trimmed = line.trim();

        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        // If indent is less than expected, we've exited this list level
        if indent < item_indent {
            break;
        }

        // Skip lines that are deeper (continuation of previous item)
        if indent > item_indent {
            i += 1;
            continue;
        }

        // At the exact item_indent: must start with "- "
        if !trimmed.starts_with("- ") {
            break;
        }

        let content = &trimmed[2..]; // After "- "

        // Check if the list item is an array
        if content.starts_with('[') {
            if let Some(header) = parse_array_header(content) {
                let arr = parse_array_body(&header, lines, i, indent + 2)?;
                items.push(arr);
                i = skip_nested_lines(lines, i + 1, indent + 2);
                continue;
            }
        }

        // Check if the list item is an object (has key: pattern)
        if item_content_is_object(content) {
            let (obj, next_i) = parse_list_item_object(lines, i, indent + 2, content)?;
            items.push(obj);
            i = next_i;
            continue;
        }

        // Primitive value
        items.push(parse_primitive_value(content)?);
        i += 1;
    }

    Ok(Value::Array(items))
}

/// Heuristic: does the content after "- " look like an object field (key: value)?
/// Checks for quoted key, unquoted `key:`, or `key[N]` patterns.
fn item_content_is_object(content: &str) -> bool {
    // Check if content starts with a key: pattern
    if content.starts_with('"') {
        if let Some(end) = find_closing_quote(content, 1) {
            return end + 1 < content.len() && content.as_bytes()[end + 1] == b':';
        }
        return false;
    }
    // Look for key: or key[N] pattern
    if let Some(pos) = content.find(':') {
        let before = &content[..pos];
        return !before.contains(' ') && !before.is_empty();
    }
    if let Some(pos) = content.find('[') {
        let before = &content[..pos];
        return !before.contains(' ') && !before.is_empty();
    }
    false
}

/// Parse an object that starts as a list item (`- key: val`).
///
/// The first field's key-value is on the "- " line itself. Subsequent sibling fields
/// appear at `hyphen_content_indent` (the indent of the content after "- ").
/// Returns the parsed object and the next line index after this item.
fn parse_list_item_object(
    lines: &[&str],
    start_line: usize,
    hyphen_content_indent: usize,
    first_field_content: &str,
) -> Result<(Value, usize)> {
    let mut map = Map::new();

    // Parse the first field from the "- key: value" line
    let mut i = parse_key_value_into_map(
        first_field_content,
        &mut map,
        lines,
        start_line,
        hyphen_content_indent,
    )?;

    let sibling_indent = hyphen_content_indent;

    while i < lines.len() {
        let line = lines[i];
        let indent = count_indent(line);
        let trimmed = line.trim();

        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        // Sibling fields are at the same indent as the hyphen content
        if indent != sibling_indent {
            break;
        }

        // Must look like a key-value pair
        if !line_has_key_colon(trimmed) && !trimmed.contains('[') {
            break;
        }

        i = parse_key_value_into_map(trimmed, &mut map, lines, i, indent)?;
    }

    Ok((Value::Object(map), i))
}

/// Skip past an array body in the line stream.
///
/// This is distinct from `skip_nested_lines` because expanded list arrays have a
/// subtle boundary condition: a line at `first_line_indent` that does NOT start with
/// "- " is a sibling field, not part of the array body. `skip_nested_lines` would
/// incorrectly consume it.
///
/// For tabular/non-list arrays, falls back to `skip_nested_lines`.
fn skip_array_body(lines: &[&str], start: usize, base_indent: usize) -> usize {
    if start >= lines.len() {
        return start;
    }

    // Detect the indent of the first non-empty line
    let mut first_line_indent = base_indent + 2;
    let mut is_list = false;
    for line in &lines[start..] {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        first_line_indent = count_indent(line);
        is_list = trimmed.starts_with("- ");
        break;
    }

    if !is_list {
        // Tabular or other: skip lines at first_line_indent or deeper
        return skip_nested_lines(lines, start, first_line_indent);
    }

    // List array: skip "- " items at first_line_indent and their deeper content
    let mut i = start;
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();
        if trimmed.is_empty() {
            i += 1;
            continue;
        }
        let indent = count_indent(line);
        if indent < first_line_indent {
            break;
        }
        if indent == first_line_indent && !trimmed.starts_with("- ") {
            // At the list item indent but not a list item — this is a sibling field
            break;
        }
        i += 1;
    }
    i
}

/// Skip lines at or deeper than `base_indent`. Stops at the first line that's
/// shallower. Used for tabular rows and nested object blocks.
fn skip_nested_lines(lines: &[&str], start: usize, base_indent: usize) -> usize {
    let mut i = start;
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();
        if trimmed.is_empty() {
            i += 1;
            continue;
        }
        let indent = count_indent(line);
        if indent < base_indent {
            break;
        }
        i += 1;
    }
    i
}

/// Parse a key-value pair from `content` and insert into `map`.
///
/// **Returns the next line index** after this key-value's content (including any
/// array body or nested object lines). This is critical for correct line advancement
/// in callers — without it, array bodies inside list item objects would cause sibling
/// fields to be swallowed.
///
/// Handles four value forms:
/// - `key[N]...` → array (inline, tabular, or expanded)
/// - `key:` → empty object or nested object (check next-line indent)
/// - `key: value` → primitive value
fn parse_key_value_into_map(
    content: &str,
    map: &mut Map<String, Value>,
    lines: &[&str],
    line_idx: usize,
    base_indent: usize,
) -> Result<usize> {
    let (key, rest) = parse_key_from_content(content)?;

    // Check for array field: key[N]...
    if rest.starts_with('[') {
        // Build a synthetic line "x[N]..." so parse_array_header can parse it
        let arr_line = format!("x{}", rest);
        if let Some(header) = parse_array_header(&arr_line) {
            let is_empty = header.len == 0;
            let is_inline = header.inline_values.is_some();
            let arr = parse_array_body(&header, lines, line_idx, base_indent)?;
            map.insert(key, arr);
            // For empty or inline arrays, no body lines to skip
            if is_empty || is_inline {
                return Ok(line_idx + 1);
            }
            // For expanded/tabular arrays, skip past the body
            let next = skip_array_body(lines, line_idx + 1, base_indent);
            return Ok(next);
        }
    }

    // rest starts with ":" for objects/empty or ": " for values
    if rest == ":" {
        // Could be empty object or object with children on next lines
        let child_indent = base_indent + 2;
        if line_idx + 1 < lines.len() {
            let next_indent = count_indent(lines[line_idx + 1]);
            if next_indent >= child_indent && !lines[line_idx + 1].trim().is_empty() {
                // Nested object
                let end = find_block_end(lines, line_idx + 1, child_indent);
                let obj = parse_object_from_lines(lines, child_indent, line_idx + 1, end)?;
                map.insert(key, obj);
                return Ok(end);
            }
        }
        // Empty object
        map.insert(key, Value::Object(Map::new()));
    } else if let Some(value_str) = rest.strip_prefix(": ") {
        let value = parse_primitive_value(value_str)?;
        map.insert(key, value);
    } else {
        // Shouldn't happen with well-formed TOON
        map.insert(key, Value::Null);
    }

    Ok(line_idx + 1)
}

/// Parse a key from the beginning of content, returning `(key, rest_after_key)`.
///
/// For unquoted keys, finds the earliest of `:` or `[` to handle both `key: val`
/// and `key[N]: ...` patterns. Using `.find(':').or_else(|| .find('['))` would fail
/// for cases like `items[2]:` where `:` appears after `[`.
fn parse_key_from_content(content: &str) -> Result<(String, String)> {
    if content.starts_with('"') {
        // Quoted key
        let end = find_closing_quote(content, 1).ok_or_else(|| ToonError::ToonParse {
            line: 0,
            message: "Unterminated quoted key".to_string(),
        })?;
        let key = unescape_string(&content[1..end]);
        let rest = content[end + 1..].to_string();
        Ok((key, rest))
    } else {
        // Unquoted key — find the earliest of ':' or '['
        let colon_pos = content.find(':');
        let bracket_pos = content.find('[');
        let end = match (colon_pos, bracket_pos) {
            (Some(c), Some(b)) => c.min(b),
            (Some(c), None) => c,
            (None, Some(b)) => b,
            (None, None) => content.len(),
        };
        let key = content[..end].to_string();
        let rest = content[end..].to_string();
        Ok((key, rest))
    }
}

/// Parse an object from indented lines
fn parse_object_from_lines(
    lines: &[&str],
    expected_indent: usize,
    start: usize,
    end: usize,
) -> Result<Value> {
    let mut map = Map::new();
    let mut i = start;

    while i < end {
        let line = lines[i];
        let trimmed = line.trim();

        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        let indent = count_indent(line);
        if indent < expected_indent {
            break;
        }
        if indent > expected_indent {
            // This is a child line of a previous key — skip
            i += 1;
            continue;
        }

        // At our indent level — parse as key-value
        i = parse_key_value_into_map(trimmed, &mut map, lines, i, indent)?;
        // Skip any nested content that parse_key_value_into_map didn't consume
        while i < end {
            let next_line = lines[i];
            let next_trimmed = next_line.trim();
            if next_trimmed.is_empty() {
                i += 1;
                continue;
            }
            let next_indent = count_indent(next_line);
            if next_indent <= expected_indent {
                break;
            }
            i += 1;
        }
    }

    Ok(Value::Object(map))
}

/// Find the end of a block at the given indent level
fn find_block_end(lines: &[&str], start: usize, min_indent: usize) -> usize {
    let mut i = start;
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();
        if trimmed.is_empty() {
            i += 1;
            continue;
        }
        let indent = count_indent(line);
        if indent < min_indent {
            break;
        }
        i += 1;
    }
    i
}

/// Parse a primitive value from a string token
fn parse_primitive_value(s: &str) -> Result<Value> {
    Ok(parse_primitive_token(s))
}

/// Parse an unquoted or quoted token into a JSON Value.
///
/// Type inference order: quoted string → null → bool → integer → float → unquoted string.
/// This mirrors the encoder's quoting rules: strings that look like numbers/bools are
/// quoted by the encoder, so unquoted tokens can be safely interpreted as their types.
fn parse_primitive_token(s: &str) -> Value {
    let s = s.trim();

    // Quoted string
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        let inner = &s[1..s.len() - 1];
        return Value::String(unescape_string(inner));
    }

    // null
    if s == "null" {
        return Value::Null;
    }

    // bool
    if s == "true" {
        return Value::Bool(true);
    }
    if s == "false" {
        return Value::Bool(false);
    }

    // Try integer
    if let Ok(n) = s.parse::<i64>() {
        return Value::Number(n.into());
    }

    // Try float
    if let Ok(f) = s.parse::<f64>() {
        if let Some(n) = serde_json::Number::from_f64(f) {
            return Value::Number(n);
        }
    }

    // Default: unquoted string
    Value::String(s.to_string())
}

/// Count leading spaces in a line (each 2 spaces = 1 indent level)
fn count_indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// Find the position of the closing quote, handling escape sequences
fn find_closing_quote(s: &str, start: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut i = start;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2; // Skip escaped character
        } else if bytes[i] == b'"' {
            return Some(i);
        } else {
            i += 1;
        }
    }
    None
}

/// Unescape a TOON string (handle \\, \", \n, \r, \t)
fn unescape_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some('\\') => out.push('\\'),
                Some('"') => out.push('"'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}
