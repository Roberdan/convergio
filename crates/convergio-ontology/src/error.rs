//! Error type for the ontology runtime.

use thiserror::Error;

/// Convenience alias for results from this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors produced by the ontology runtime.
#[derive(Debug, Error)]
pub enum Error {
    /// Underlying SQLite/sqlx failure.
    #[error("sqlx error: {0}")]
    Sqlx(#[from] sqlx::Error),

    /// Migration runner failure.
    #[error("migrate error: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),

    /// Serialization failure when computing the canonical body
    /// payload or content hash.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// One of the persisted timestamps was not RFC3339.
    #[error("timestamp parse error: {0}")]
    TimestampParse(String),

    /// Audit writer/verifier error from `convergio-durability`.
    #[error("audit error: {0}")]
    Audit(#[from] convergio_durability::DurabilityError),

    /// Caller asked for a schema_version that already exists with a
    /// different `content_hash`. The registry is append-on-write per
    /// ADR-0053; bump the version to land a new revision.
    #[error("conflict: {kind} `{name}` already has schema_version {version} with a different content_hash")]
    VersionConflict {
        /// Type family (object / link / property).
        kind: &'static str,
        /// Registry name of the conflicting entry.
        name: String,
        /// The schema_version that already exists.
        version: i64,
    },

    /// An import payload referenced object types it did not define.
    #[error("import closure error: undefined references: {missing}")]
    ImportClosure {
        /// Semicolon-joined list of the missing references.
        missing: String,
    },

    /// Caller asked for an entry that does not exist.
    #[error("not found: {kind} `{name}`")]
    NotFound {
        /// Type family (object / link / property).
        kind: &'static str,
        /// Registry name that was not found.
        name: String,
    },

    /// A referenced object instance does not exist under the given tenant.
    #[error("object instance not found in tenant: tenant_id={tenant_id} id={id}")]
    InstanceNotFound {
        /// Tenant scope.
        tenant_id: String,
        /// Missing object id.
        id: String,
    },

    /// Requested ontology branch does not exist.
    #[error("ontology branch not found: {id}")]
    BranchNotFound {
        /// Missing branch id.
        id: String,
    },

    /// Branch metadata is invalid.
    #[error("invalid ontology branch: {reason}")]
    InvalidBranch {
        /// Validation failure reason.
        reason: String,
    },

    /// Ontology entry payload is invalid.
    #[error("invalid ontology entry: {reason}")]
    InvalidEntry {
        /// Validation failure reason.
        reason: String,
    },

    /// Feature exists on the API surface but its underlying primitive
    /// has not landed yet. The W1 `branch-diff` command returns this
    /// because branching itself ships in a later ADR (ADR-0059).
    #[error("not implemented yet: {feature}")]
    NotImplemented {
        /// Stable identifier for the missing feature.
        feature: &'static str,
    },
}
