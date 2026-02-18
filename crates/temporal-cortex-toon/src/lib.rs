//! # temporal-cortex-toon
//!
//! Pure-Rust encoder and decoder for **TOON (Token-Oriented Object Notation)** v3.0.
//!
//! TOON is a compact, human-readable serialization format designed to reduce LLM token
//! consumption when processing structured data. It achieves this through key folding
//! (indentation instead of braces), tabular compression for uniform arrays, and
//! context-dependent quoting that eliminates unnecessary quote tokens.
//!
//! ## Quick start
//!
//! ```rust
//! use toon_core::{encode, decode};
//!
//! // JSON → TOON
//! let json = r#"{"name":"Alice","scores":[95,87,92]}"#;
//! let toon = encode(json).unwrap();
//! assert_eq!(toon, "name: Alice\nscores[3]: 95,87,92");
//!
//! // TOON → JSON (roundtrip)
//! let back = decode(&toon).unwrap();
//! assert_eq!(back, json);
//! ```
//!
//! ## Modules
//!
//! - [`encoder`] — JSON string → TOON string
//! - [`decoder`] — TOON string → JSON string
//! - [`filter`] — Semantic filtering + TOON encode (`filter_and_encode`, `CalendarFilter`)
//! - [`error`] — Error types for parse/encode failures
//! - [`types`] — `ToonValue` AST (reserved for future direct-manipulation use)

pub mod decoder;
pub mod encoder;
pub mod error;
pub mod filter;
pub mod types;

pub use decoder::decode;
pub use encoder::encode;
pub use error::ToonError;
pub use filter::{filter_and_encode, filter_fields, CalendarFilter};
