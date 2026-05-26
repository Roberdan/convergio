//! Small typed wrappers for connector identifiers and cursors.

use serde::{Deserialize, Serialize};

/// Stable connector identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ConnectorId(pub String);

impl ConnectorId {
    /// Create a new connector id.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// Opaque watermark cursor carried across pull runs.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Watermark(pub String);

impl Watermark {
    /// Wrap a watermark string.
    pub fn new(v: impl Into<String>) -> Self {
        Self(v.into())
    }
}

/// Stable hash of a connector's mapping schema.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SchemaHash(pub String);

impl SchemaHash {
    /// Create from a hex string.
    pub fn new_hex(hex: impl Into<String>) -> Self {
        Self(hex.into())
    }

    /// Borrow the hex representation.
    pub fn as_hex(&self) -> &str {
        &self.0
    }
}
