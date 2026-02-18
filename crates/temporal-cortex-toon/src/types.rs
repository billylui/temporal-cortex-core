//! TOON value types for direct AST manipulation (reserved for future use).
//!
//! Currently, encoding and decoding go through `serde_json::Value` as the
//! intermediate representation. This module defines a TOON-native AST that
//! could be used for direct manipulation without the JSON roundtrip, e.g.,
//! for semantic filtering or streaming transformations.

/// Represents a TOON document value. Mirrors JSON types but separates integers
/// from floats (TOON preserves the distinction) and uses `Vec<(String, ToonValue)>`
/// for objects to maintain insertion order without depending on `IndexMap`.
#[derive(Debug, Clone, PartialEq)]
pub enum ToonValue {
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Array(Vec<ToonValue>),
    /// Key-value pairs in insertion order.
    Object(Vec<(String, ToonValue)>),
}
