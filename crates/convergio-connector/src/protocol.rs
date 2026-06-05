//! Line-delimited JSON protocol for sandboxed connectors.

use crate::error::{ConnectorError, FailureKind};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Operation name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Op {
    /// Discover available streams.
    Discover,
    /// Pull a page of records.
    Pull,
    /// Get current watermark.
    Watermark,
    /// Get schema hash.
    SchemaHash,
    /// Health check.
    Health,
}

/// One request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    /// Correlation id.
    pub id: String,
    /// Operation.
    pub op: Op,
    /// Operation-specific params.
    #[serde(default)]
    pub params: Value,
}

/// One response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Response {
    /// Correlation id.
    pub id: String,
    /// Whether the operation succeeded.
    pub ok: bool,
    /// Operation result object when `ok`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// Error object when `!ok`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<ProtocolError>,
}

/// Structured error returned over the protocol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolError {
    /// Retry classification.
    pub kind: FailureKindWire,
    /// Human-readable message.
    pub message: String,
}

/// Wire representation for [`FailureKind`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureKindWire {
    /// Retryable.
    Retryable,
    /// Fatal.
    Fatal,
}

impl FailureKindWire {
    /// Convert the wire enum into the crate classification.
    pub fn into_kind(self) -> FailureKind {
        match self {
            Self::Retryable => FailureKind::Retryable,
            Self::Fatal => FailureKind::Fatal,
        }
    }
}

impl Response {
    /// Convert a wire response into an application-level result.
    ///
    /// - `ok=true` yields `result` (or `null` when absent)
    /// - `ok=false` yields [`ConnectorError::ConnectorFailed`]
    pub fn into_result(self) -> Result<Value, ConnectorError> {
        if self.ok {
            return Ok(self.result.unwrap_or(Value::Null));
        }
        let err = self.error.unwrap_or(ProtocolError {
            kind: FailureKindWire::Fatal,
            message: "missing error body".to_string(),
        });
        Err(ConnectorError::ConnectorFailed {
            kind: err.kind.into_kind(),
            message: err.message,
        })
    }
}
