use crate::{SchemaVersion, TypeId};

/// Policy errors when registering a new schema version.
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    /// The provided version does not match the expected bump.
    #[error("invalid version bump for {kind}:{id}: last={last}, expected={expected}, got={got}")]
    InvalidVersionBump {
        /// Kind label (`object`, `link`, `property`).
        kind: &'static str,
        /// Stable identifier.
        id: TypeId,
        /// Previous highest version.
        last: SchemaVersion,
        /// Expected next version.
        expected: SchemaVersion,
        /// Provided version.
        got: SchemaVersion,
    },

    /// A breaking change was registered without a migration plan reference.
    #[error("breaking change requires migration plan reference: {label}")]
    BreakingRequiresMigration {
        /// Kind+id label.
        label: String,
    },

    /// The registry is missing a prior version needed for comparison.
    #[error("missing prior version for {kind}:{id}@{version}")]
    MissingPriorVersion {
        /// Kind label (`object`, `link`, `property`).
        kind: &'static str,
        /// Stable identifier.
        id: TypeId,
        /// Missing version.
        version: SchemaVersion,
    },

    /// Serialization failed while computing the content hash.
    #[error("failed to serialize schema spec: {0}")]
    Serialize(serde_json::Error),
}
