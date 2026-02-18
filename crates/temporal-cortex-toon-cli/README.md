# temporal-cortex-toon-cli

CLI tool for encoding, decoding, and analyzing [TOON](https://crates.io/crates/temporal-cortex-toon) (Token-Oriented Object Notation) files.

## Installation

```bash
cargo install temporal-cortex-toon-cli
```

## Usage

```bash
# Encode JSON to TOON (stdin to stdout)
echo '{"name":"Alice","age":30}' | toon encode

# Encode from file to file
toon encode -i data.json -o data.toon

# Encode with field filtering (strip noisy fields before encoding)
echo '{"name":"Event","etag":"abc"}' | toon encode --filter etag

# Encode with Google Calendar preset filter
toon encode --filter-preset google -i calendar.json

# Decode TOON back to pretty-printed JSON
toon decode -i data.toon

# Show compression statistics
toon stats -i data.json
```

## What is TOON?

TOON is a compact, human-readable format that minimizes token usage when feeding structured data to LLMs. It achieves 50%+ token reduction vs JSON through key folding, tabular arrays, and inline arrays while maintaining perfect roundtrip fidelity.

## License

MIT OR Apache-2.0
