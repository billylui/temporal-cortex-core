# @temporal-cortex/toon

WASM-powered TOON (Token-Oriented Object Notation) encoder/decoder for Node.js.

This package wraps the Rust `temporal-cortex-toon` library via WebAssembly, providing near-native performance for TOON encoding and decoding in JavaScript/TypeScript environments.

## Installation

```bash
npm install @temporal-cortex/toon
# or
pnpm add @temporal-cortex/toon
```

## Usage

```typescript
import { encode, decode } from "@temporal-cortex/toon";

// JSON string → TOON string
const toon = encode('{"name":"Alice","scores":[95,87,92]}');
console.log(toon);
// name: Alice
// scores[3]: 95,87,92

// TOON string → JSON string (perfect roundtrip)
const json = decode(toon);
console.log(json);
// {"name":"Alice","scores":[95,87,92]}
```

## API

### `encode(json: string): string`

Converts a valid JSON string into TOON format. Throws if the input is not valid JSON.

### `decode(toon: string): string`

Converts a TOON string back into compact JSON. Throws if the input is not valid TOON.

## Build from Source

This package requires the WASM artifacts to be built from the Rust crate first:

```bash
# From the monorepo root:

# 1. Build the WASM binary
cargo build -p temporal-cortex-toon-wasm --target wasm32-unknown-unknown --release

# 2. Generate Node.js bindings
wasm-bindgen --target nodejs \
  --out-dir packages/temporal-cortex-toon-js/wasm/ \
  target/wasm32-unknown-unknown/release/toon_wasm.wasm

# 3. Rename for ESM/CJS compatibility
mv packages/temporal-cortex-toon-js/wasm/toon_wasm.js packages/temporal-cortex-toon-js/wasm/toon_wasm.cjs

# 4. Build TypeScript
pnpm --filter @temporal-cortex/toon build

# 5. Run tests
pnpm --filter @temporal-cortex/toon test
```

## Architecture

```
src/index.ts          ← Public API (encode/decode), loads WASM via createRequire
wasm/toon_wasm.cjs    ← wasm-bindgen generated CommonJS bindings
wasm/toon_wasm.wasm   ← Compiled WASM binary from temporal-cortex-toon (Rust)
wasm/toon_wasm.d.ts   ← TypeScript type declarations for WASM exports
```

The package uses `createRequire(import.meta.url)` to load the CommonJS WASM bindings from an ESM context. This bridges the module system mismatch since `wasm-bindgen --target nodejs` generates CommonJS but the package uses `"type": "module"`.

## Testing

26 tests covering encode, decode, and roundtrip operations:

```bash
pnpm --filter @temporal-cortex/toon test
```

## License

MIT OR Apache-2.0
