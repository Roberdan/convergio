//! Error types for convergio-parse-multi.

use thiserror::Error;

/// Errors produced by the parsing layer.
#[derive(Debug, Error)]
pub enum ParseError {
    /// tree-sitter returned an error node at the root.
    #[error("parse error in {file}: tree-sitter reported syntax errors")]
    SyntaxError {
        /// Source file path.
        file: String,
    },

    /// Source bytes are not valid UTF-8.
    #[error("source file {file} is not valid UTF-8: {source}")]
    Encoding {
        /// Source file path.
        file: String,
        /// Underlying UTF-8 error.
        #[source]
        source: std::str::Utf8Error,
    },

    /// tree-sitter parser failed to produce a tree (internal error).
    #[error("tree-sitter failed to parse {file}")]
    ParserFailed {
        /// Source file path.
        file: String,
    },
}

/// Alias for `Result<T, ParseError>`.
pub type Result<T> = std::result::Result<T, ParseError>;
