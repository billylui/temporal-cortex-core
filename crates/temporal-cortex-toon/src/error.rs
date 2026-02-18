//! Error types for TOON encoding and decoding operations.

use thiserror::Error;

/// Errors that can occur during TOON encoding or decoding.
#[derive(Error, Debug)]
pub enum ToonError {
    /// The input string was not valid JSON (encoding path).
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),

    /// The input string was not valid TOON (decoding path).
    /// Includes the 1-based line number where the error was detected.
    #[error("TOON parse error at line {line}: {message}")]
    ToonParse { line: usize, message: String },

    /// A structural error during encoding (e.g., unsupported value type).
    #[error("Encoding error: {0}")]
    Encode(String),
}

/// Convenience alias used throughout temporal-cortex-toon.
pub type Result<T> = std::result::Result<T, ToonError>;
