//! Errors for Layer 3.

use thiserror::Error;

/// All errors the supervisor can produce.
#[derive(Debug, Error)]
pub enum LifecycleError {
    /// Process row not found.
    #[error("process not found: {0}")]
    NotFound(String),

    /// `tokio::process::Command::spawn` failed.
    #[error("spawn failed: {0}")]
    SpawnFailed(String),

    /// Spawn bookkeeping exceeded the configured timeout.
    #[error("spawn timed out after {timeout_ms}ms: {command}")]
    SpawnTimedOut {
        /// Command being spawned.
        command: String,
        /// Timeout in milliseconds.
        timeout_ms: u128,
    },

    /// Persisted timestamp data could not be parsed.
    #[error("invalid timestamp in {field}: {value}")]
    InvalidTimestamp {
        /// Column name containing the bad timestamp.
        field: &'static str,
        /// Persisted value that failed RFC3339 parsing.
        value: String,
    },

    /// Persisted status string did not match any known
    /// [`crate::ProcessStatus`] variant — database invariant drift
    /// that operators must see, not silently coerce away.
    #[error("invalid status in {field}: {value}")]
    InvalidStatus {
        /// Column name carrying the bad status string.
        field: &'static str,
        /// Persisted value that failed to parse.
        value: String,
    },

    /// I/O error during process management.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Underlying database error.
    #[error(transparent)]
    Db(#[from] convergio_db::DbError),

    /// Sqlx error.
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    /// Migration runner failure.
    #[error(transparent)]
    Migrate(#[from] sqlx::migrate::MigrateError),
}

/// Convenience alias.
pub type Result<T, E = LifecycleError> = std::result::Result<T, E>;
