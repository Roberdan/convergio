//! Errors for `convergio-embed`.

use thiserror::Error;

/// All errors the embed store can produce.
#[derive(Debug, Error)]
pub enum EmbedError {
    /// Underlying database error from `convergio-db`.
    #[error(transparent)]
    Db(#[from] convergio_db::DbError),

    /// `sqlx` error not surfaced through `convergio-db`.
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    /// Migration failure.
    #[error(transparent)]
    Migrate(#[from] sqlx::migrate::MigrateError),

    /// Vector dimension mismatch between the stored row and the
    /// caller's expectation.
    #[error("vector dim mismatch: expected {expected}, stored {got}")]
    DimMismatch {
        /// Dimension declared by the caller (or the embedder model).
        expected: usize,
        /// Dimension reconstructed from the stored blob length.
        got: usize,
    },

    /// Stored blob length is not a multiple of 4 (the size of `f32`).
    #[error("corrupt embedding blob: length {0} is not a multiple of 4")]
    CorruptBlob(usize),

    /// The configured [`crate::Embedder`] refused this input. Carries
    /// the underlying message so the caller can surface or skip.
    /// Lifted out of [`crate::EmbedderError`] at the
    /// `convergio-embed` boundary so [`crate::ingest::ingest`] can
    /// distinguish recoverable embedder failures from store errors.
    #[error("embedder failed: {0}")]
    EmbedderFailed(String),
}

/// Convenience alias for the embed crate.
pub type Result<T, E = EmbedError> = std::result::Result<T, E>;
