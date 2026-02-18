# temporal-cortex-toon

Pure-Rust encoder and decoder for **TOON (Token-Oriented Object Notation)** v3.0.

TOON is a compact, human-readable serialization format designed to reduce LLM token consumption when processing structured data. It achieves this through key folding, tabular compression, and context-dependent quoting.

## Usage

```rust
use toon_core::{encode, decode};

// JSON → TOON
let json = r#"{"name":"Alice","scores":[95,87,92]}"#;
let toon = encode(json).unwrap();
assert_eq!(toon, "name: Alice\nscores[3]: 95,87,92");

// TOON → JSON (perfect roundtrip)
let back = decode(&toon).unwrap();
assert_eq!(back, json);
```

## TOON Format Overview

### Primitives and Objects

```
name: Alice          ← unquoted string (no ambiguity)
age: 30              ← number
active: true         ← boolean
id: "42"             ← quoted (looks numeric, must preserve string type)
address:             ← nested object (indentation, no braces)
  city: Portland
  state: OR
empty:               ← empty object
```

### Arrays

Three representations, chosen automatically for maximum compression:

**Inline** (all primitives):
```
tags[3]: rust,wasm,llm
```

**Tabular** (uniform objects with identical primitive-only keys):
```
attendees[2]{email,status}:
  alice@co.com,accepted
  bob@co.com,tentative
```

**Expanded** (mixed/complex content):
```
items[2]:
  - kind: event
    summary: Standup
  - kind: event
    summary: Sprint Planning
```

### Quoting Rules

Strings are only quoted when they would be ambiguous:

| Condition | Example | Encoded as |
|-----------|---------|-----------|
| Looks like bool | `true` | `"true"` |
| Looks like null | `null` | `"null"` |
| Looks like number | `42` | `"42"` |
| Contains colon (in document context) | `10:30 AM` | `"10:30 AM"` |
| Contains comma (in inline/tabular context) | `a, b` | `"a, b"` |
| Empty string | | `""` |
| Leading/trailing whitespace | ` hello ` | `" hello "` |
| Contains brackets/braces | `[1]` | `"[1]"` |
| Starts with hyphen | `-foo` | `"-foo"` |

## Architecture

```
encoder.rs  ← JSON string → serde_json::Value → TOON string
decoder.rs  ← TOON string → serde_json::Value → JSON string
error.rs    ← ToonError enum (JsonParse, ToonParse, Encode)
types.rs    ← ToonValue AST (reserved for future direct manipulation)
lib.rs      ← Public API: encode(), decode(), ToonError
```

The encoder walks the `serde_json::Value` tree and selects the most compact TOON representation for each node. The decoder parses indentation-based TOON structure back into a `serde_json::Value`.

Key implementation detail: `serde_json` must use the `preserve_order` feature (enabled in workspace `Cargo.toml`) to maintain JSON key insertion order via `IndexMap`.

## Testing

164 tests across three suites:

- **59 encoder tests** — primitives, objects, arrays (inline/tabular/expanded), nesting, quoting edge cases
- **63 decoder tests** — mirrors encoder tests plus string type inference, escape sequences, calendar-realistic data
- **42 roundtrip tests** — `decode(encode(json)) == json` for all value types

```bash
cargo test -p temporal-cortex-toon
```

## License

MIT OR Apache-2.0
