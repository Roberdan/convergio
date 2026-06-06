//! Error type for the ontology-author pipeline.

use std::path::PathBuf;

/// Errors raised while authoring an ontology draft.
#[derive(Debug, thiserror::Error)]
pub enum AuthorError {
    /// The request carried neither an intent nor any documents.
    #[error("authoring request is empty: provide an intent and/or at least one document")]
    EmptyRequest,

    /// A source document could not be read or converted.
    #[error("failed to convert document {path}: {reason}")]
    DocConversion {
        /// The offending document path.
        path: PathBuf,
        /// Human-readable failure reason.
        reason: String,
    },

    /// The proposer (vendor CLI or stub) failed to produce output.
    #[error("ontology proposer failed: {0}")]
    Proposer(String),

    /// The proposer output was not valid JSON for the draft schema.
    #[error("proposer output was not valid draft JSON: {0}")]
    Parse(String),

    /// Validation still failed after the configured repair attempts.
    #[error("draft did not become valid after {attempts} attempt(s); {count} violation(s) remain")]
    Unrepaired {
        /// Number of proposer attempts made.
        attempts: u32,
        /// Number of violations remaining on the final attempt.
        count: usize,
    },

    /// Converting the draft into ontology records failed.
    #[error("record construction failed: {0}")]
    Records(String),

    /// An artifact could not be written to disk.
    #[error("failed to write artifact {path}: {reason}")]
    Write {
        /// The artifact path that failed.
        path: PathBuf,
        /// Human-readable failure reason.
        reason: String,
    },

    /// A wrapped ontology-crate error from the exporters.
    #[error("ontology export error: {0}")]
    Ontology(String),

    /// A wrapped provenance-crate error.
    #[error("provenance error: {0}")]
    Provenance(String),
}

/// Convenient result alias for this crate.
pub type Result<T> = std::result::Result<T, AuthorError>;
