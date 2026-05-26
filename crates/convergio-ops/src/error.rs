//! Errors for `convergio-ops`.

use thiserror::Error;

/// All errors the ops workflow engine layer can produce.
#[derive(Debug, Error)]
pub enum OpsError {
    /// A row that should exist does not.
    #[error("not found: {entity} id={id}")]
    NotFound {
        /// Logical entity name.
        entity: &'static str,
        /// Identifier the caller passed.
        id: String,
    },

    /// Workflow spec failed validation.
    #[error("invalid workflow spec: {reason}")]
    InvalidSpec {
        /// Validation failure reason.
        reason: String,
    },

    /// Underlying database error.
    #[error(transparent)]
    Db(#[from] convergio_db::DbError),

    /// Sqlx error not surfaced via `convergio-db`.
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    /// JSON serialization / deserialization error.
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    /// Migration runner failure.
    #[error(transparent)]
    Migrate(#[from] sqlx::migrate::MigrateError),
}

/// Convenience alias.
pub type Result<T, E = OpsError> = std::result::Result<T, E>;
