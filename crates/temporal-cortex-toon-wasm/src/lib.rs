//! WASM bindings for temporal-cortex-toon.
//!
//! Exposes `encode` and `decode` as `#[wasm_bindgen]` functions that can be
//! called from JavaScript/TypeScript. Built with `wasm-bindgen-cli` (not
//! wasm-pack, which was archived in July 2025).
//!
//! ## Build process
//!
//! ```sh
//! cargo build -p temporal-cortex-toon-wasm --target wasm32-unknown-unknown --release
//! wasm-bindgen --target nodejs --out-dir packages/temporal-cortex-toon-js/wasm/ \
//!   target/wasm32-unknown-unknown/release/toon_wasm.wasm
//! # Rename .js → .cjs for ESM compatibility
//! mv packages/temporal-cortex-toon-js/wasm/toon_wasm.js packages/temporal-cortex-toon-js/wasm/toon_wasm.cjs
//! ```
//!
//! The generated `.cjs` file is loaded by `@temporal-cortex/toon` (the NPM package)
//! via `createRequire` to bridge CommonJS/ESM module systems.

use wasm_bindgen::prelude::*;

/// Encode a JSON string into TOON v3.0 format.
///
/// Returns the TOON string, or throws a JS error if the input is not valid JSON.
#[wasm_bindgen]
pub fn encode(json: &str) -> std::result::Result<String, JsValue> {
    toon_core::encode(json).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Decode a TOON string back into compact JSON format.
///
/// Returns the JSON string, or throws a JS error if the input is not valid TOON.
#[wasm_bindgen]
pub fn decode(toon: &str) -> std::result::Result<String, JsValue> {
    toon_core::decode(toon).map_err(|e| JsValue::from_str(&e.to_string()))
}
