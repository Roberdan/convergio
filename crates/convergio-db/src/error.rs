//! Errors returned by the database abstraction.

use thiserror::Error;

/// All errors that can be raised by [`crate::Pool`].
#[derive(Debug, Error)]
pub enum DbError {
    /// The provided URL scheme is not supported in this build.
    #[error("unsupported database URL scheme: {0}")]
    UnsupportedScheme(String),

    /// Failure parsing the database URL.
    #[error("invalid database URL: {0}")]
    InvalidUrl(String),

    /// Connection or query error from sqlx.
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    /// Migration failure.
    #[error(transparent)]
    Migrate(#[from] sqlx::migrate::MigrateError),

    /// I/O error (e.g. creating the SQLite parent directory).
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Failed to register sqlite-vec (`vec0`) as a SQLite auto-extension.
    #[error("sqlite-vec auto-extension registration failed (sqlite rc {0})")]
    SqliteVecAutoExtension(i32),
}

/// Convenience alias.
pub type Result<T, E = DbError> = std::result::Result<T, E>;
