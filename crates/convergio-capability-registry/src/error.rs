//! Error type for the remote capability registry layer.

use std::io;

/// All fallible operations in this crate return [`Result`].
pub type Result<T> = std::result::Result<T, RegistryError>;

/// Errors emitted while talking to a remote capability registry or
/// loading the local trust store.
///
/// Variants are intentionally narrow so the daemon (F2) can map each
/// failure mode to a stable refusal reason in audit rows.
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    /// HTTP-level failure (DNS, TLS handshake, non-2xx status, etc.).
    #[error("network: {0}")]
    Network(String),

    /// A response decoded successfully but failed schema validation.
    #[error("invalid response from {endpoint}: {reason}")]
    InvalidResponse {
        /// Origin that returned the malformed document.
        endpoint: String,
        /// Human-readable validation failure.
        reason: String,
    },

    /// The requested resource was not found at the configured registry.
    #[error("not found: {0}")]
    NotFound(String),

    /// Bundle exceeded the configured size cap.
    #[error("bundle too large: {size} bytes (cap {cap})")]
    BundleTooLarge {
        /// Bytes returned by the server.
        size: u64,
        /// Cap configured on the fetcher.
        cap: u64,
    },

    /// Trust-store file could not be loaded.
    #[error("trust store: {0}")]
    TrustStore(String),

    /// Trust-store JSON was syntactically valid but rejected by validation.
    #[error("trust store entry rejected: {0}")]
    TrustStoreEntry(String),

    /// Configured registry URL is not a valid HTTPS origin.
    #[error("invalid registry url: {0}")]
    InvalidUrl(String),

    /// Local I/O failure (e.g. reading a baked-in or overlay key file).
    #[error("io: {0}")]
    Io(#[from] io::Error),

    /// JSON serialization/deserialization failure.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

impl RegistryError {
    /// Construct a [`RegistryError::Network`] from any error.
    pub fn network(err: impl std::fmt::Display) -> Self {
        Self::Network(err.to_string())
    }

    /// Construct a [`RegistryError::InvalidResponse`].
    pub fn invalid_response(endpoint: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::InvalidResponse {
            endpoint: endpoint.into(),
            reason: reason.into(),
        }
    }
}
