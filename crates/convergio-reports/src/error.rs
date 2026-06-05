//! Error types for `convergio-reports`.

use thiserror::Error;

/// Result alias for report operations.
pub type Result<T> = std::result::Result<T, ReportError>;

/// Report engine error.
#[derive(Debug, Error)]
pub enum ReportError {
    /// Database error.
    #[error("db error: {0}")]
    Db(#[from] sqlx::Error),

    /// Migration runner error.
    #[error("migration error: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),

    /// A referenced entity was not found.
    #[error("not found: {0}")]
    NotFound(String),

    /// Input did not pass semantic validation.
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// JSON Schema validation failed.
    #[error("parameter validation failed: {0}")]
    ParamValidation(String),

    /// Template rendering failed.
    #[error("template render failed: {0}")]
    Template(String),

    /// PDF compilation failed.
    #[error("pdf render failed: {0}")]
    Pdf(String),

    /// DOCX build failed.
    #[error("docx render failed: {0}")]
    Docx(String),

    /// Image/QR encoding failed.
    #[error("qr encode failed: {0}")]
    Qr(String),
}
