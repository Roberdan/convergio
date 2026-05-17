//! Error types for `convergio-fleet`.

use thiserror::Error;

/// All errors produced by `convergio-fleet`.
#[derive(Debug, Error)]
pub enum FleetError {
    /// Database error.
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),

    /// Migration error.
    #[error("migration error: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),

    /// Config parse error.
    #[error("fleet.toml parse error: {0}")]
    Config(#[from] toml::de::Error),

    /// Config serialization error.
    #[error("fleet.toml serialize error: {0}")]
    ConfigSer(#[from] toml::ser::Error),

    /// I/O error (reading fleet.toml).
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),

    /// A repo with this name already exists.
    #[error("repo '{0}' already exists in the fleet")]
    RepoDuplicate(String),

    /// A repo with this name was not found.
    #[error("repo '{0}' not found in the fleet")]
    RepoNotFound(String),

    /// Generic "not found" for fleet entities other than repos
    /// (e.g. `fleet_plan <id>`).
    #[error("not found: {0}")]
    NotFound(String),

    /// Caller supplied invalid input (empty title, malformed scope).
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// Error propagated from the embeddings layer.
    #[error("embed error: {0}")]
    Embed(#[from] convergio_embed::EmbedError),
}

/// Convenience alias.
pub type Result<T> = std::result::Result<T, FleetError>;
