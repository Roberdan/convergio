//! Connector trait surface.

use crate::error::ConnectorError;
use crate::types::{SchemaHash, Watermark};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Request to discover available streams/datasets for a connector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DiscoverRequest {
    /// Optional opaque hint for filtering, connector-specific.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
}

/// One discoverable source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoverItem {
    /// Stable stream/dataset identifier.
    pub stream: String,
    /// Human label.
    pub label: String,
}

/// Request to pull records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequest {
    /// Optional stream to pull.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream: Option<String>,
    /// Pull from this watermark (exclusive).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since: Option<Watermark>,
    /// Page size hint.
    #[serde(default = "default_limit")]
    pub limit: u32,
}

fn default_limit() -> u32 {
    100
}

/// One page of pulled records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullPage<R> {
    /// Pulled records.
    pub records: Vec<R>,
    /// Watermark representing the newest record in this page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_watermark: Option<Watermark>,
    /// Whether more pages are available.
    pub has_more: bool,
}

/// Health status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Health {
    /// Connector is healthy.
    Healthy,
    /// Connector is degraded but usable.
    Degraded,
    /// Connector is unhealthy.
    Unhealthy,
}

/// Connector interface.
#[async_trait]
pub trait Connector: Send + Sync {
    /// Record type emitted by this connector.
    type Record: Send + Sync + Serialize + for<'de> Deserialize<'de>;

    /// Discover streams/datasets.
    async fn discover(&self, req: DiscoverRequest) -> Result<Vec<DiscoverItem>, ConnectorError>;

    /// Pull a page of records.
    async fn pull(&self, req: PullRequest) -> Result<PullPage<Self::Record>, ConnectorError>;

    /// Report the current watermark (if any).
    async fn watermark(&self) -> Result<Option<Watermark>, ConnectorError>;

    /// Return the stable schema hash for this connector's mapping.
    async fn schema_hash(&self) -> Result<SchemaHash, ConnectorError>;

    /// Health check.
    async fn health(&self) -> Result<Health, ConnectorError>;
}
