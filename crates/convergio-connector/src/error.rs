//! Errors for `convergio-connector`.

use thiserror::Error;

/// Whether a connector error should be retried by the runner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureKind {
    /// A transient failure (timeouts, upstream 5xx, rate limit).
    Retryable,
    /// A non-retryable failure (bad credentials, invalid mapping).
    Fatal,
}

/// Crate-wide error type.
#[derive(Debug, Error)]
pub enum ConnectorError {
    /// Underlying I/O failure.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// YAML parsing failure.
    #[error("yaml error: {0}")]
    Yaml(String),

    /// JSON parsing / serialization failure.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// Connector protocol violation.
    #[error("protocol error: {0}")]
    Protocol(String),

    /// Connector call timed out.
    #[error("timeout after {secs}s: {what}")]
    Timeout {
        /// Timeout bound in seconds.
        secs: u64,
        /// What timed out.
        what: String,
    },

    /// Connector returned an error.
    #[error("connector failed ({kind:?}): {message}")]
    ConnectorFailed {
        /// Retry classification.
        kind: FailureKind,
        /// Connector-provided error message.
        message: String,
    },
}

impl ConnectorError {
    /// Build a YAML error from a `serde_yaml` value.
    pub fn yaml(e: serde_yaml::Error) -> Self {
        Self::Yaml(e.to_string())
    }

    /// Build a protocol error with a stable message.
    pub fn protocol(msg: impl Into<String>) -> Self {
        Self::Protocol(msg.into())
    }

    /// Build a timeout error.
    pub fn timeout(secs: u64, what: impl Into<String>) -> Self {
        Self::Timeout {
            secs,
            what: what.into(),
        }
    }
}
