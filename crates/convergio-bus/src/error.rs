//! Errors for Layer 2.

use thiserror::Error;

/// All errors the bus can produce.
#[derive(Debug, Error)]
pub enum BusError {
    /// Message id not found on ack.
    #[error("message not found: {0}")]
    NotFound(String),

    /// Topic does not match the requested scope (system vs plan-scoped).
    /// See ADR-0025.
    #[error("invalid topic scope: {0}")]
    InvalidTopicScope(String),

    /// Caller passed a non-positive page-size limit. The bus rejects
    /// `limit <= 0` because SQLite treats negative `LIMIT` as
    /// unbounded, which would expose every plan-scoped or system-scoped
    /// row in a single call (CONSTITUTION § Sacred principles —
    /// security first).
    #[error("invalid limit: {value} (must be positive)")]
    InvalidLimit {
        /// The offending value passed by the caller.
        value: i64,
    },

    /// Stored timestamp is not valid RFC 3339.
    #[error("invalid timestamp in {field}: {value}")]
    InvalidTimestamp {
        /// Column or field containing the invalid timestamp.
        field: &'static str,
        /// Raw timestamp value read from storage.
        value: String,
        /// Parser error from chrono.
        #[source]
        source: chrono::ParseError,
    },

    /// Underlying database error.
    #[error(transparent)]
    Db(#[from] convergio_db::DbError),

    /// Sqlx error not surfaced via `convergio-db`.
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    /// Migration failure.
    #[error(transparent)]
    Migrate(#[from] sqlx::migrate::MigrateError),

    /// JSON serialization error.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// Convenience alias.
pub type Result<T, E = BusError> = std::result::Result<T, E>;
