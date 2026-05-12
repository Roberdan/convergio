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

    /// `Parser::set_language` rejected the grammar — typically a
    /// tree-sitter ABI version mismatch between the runtime and a
    /// language crate. Production code must surface this as a
    /// typed error rather than panicking (P1 zero-tolerance).
    #[error("tree-sitter grammar version mismatch for {lang} (rebuild required): {source}")]
    GrammarVersionMismatch {
        /// Human-readable language tag (`rust`, `typescript`, `python`).
        lang: &'static str,
        /// Underlying tree-sitter `LanguageError`.
        #[source]
        source: tree_sitter::LanguageError,
    },
}

/// Alias for `Result<T, ParseError>`.
pub type Result<T> = std::result::Result<T, ParseError>;

#[cfg(test)]
mod tests {
    use super::*;

    /// P1 zero-tolerance regression: grammar-version-mismatch must be
    /// representable as a typed error. Pre-2026-05-12 the three
    /// production parsers used `.expect()` here. Removing the variant
    /// breaks this match and the three production call sites at the
    /// same time.
    #[test]
    fn grammar_version_mismatch_variant_signature_is_stable() {
        fn matches_variant(e: &ParseError) -> bool {
            matches!(e, ParseError::GrammarVersionMismatch { lang, .. } if !lang.is_empty())
        }
        let _ = matches_variant as fn(&ParseError) -> bool;
    }
}
