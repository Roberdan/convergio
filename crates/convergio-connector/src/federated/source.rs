//! Type-erased connector handles for the federated query path.
//!
//! A [`FederatedSource`] wraps any [`Connector`](crate::Connector) and erases
//! its associated `Record` type by serializing each pulled record into a
//! `serde_json::Value`. This is what lets a single federated result set span
//! connectors with heterogeneous record types: they all converge on one
//! common serialized form (see the module-level docs on [`crate::federated`]).

use crate::connector::{Connector, PullRequest};
use crate::error::ConnectorError;
use crate::types::ConnectorId;
use async_trait::async_trait;
use serde_json::Value;
use std::fmt;
use std::sync::Arc;

/// Object-safe view over a connector that pulls records as `serde_json::Value`.
///
/// The associated `Record` type on [`Connector`] makes `dyn Connector`
/// unusable directly; this trait erases it so heterogeneous connectors can be
/// stored side by side behind `Arc<dyn ErasedSource>`.
#[async_trait]
trait ErasedSource: Send + Sync {
    /// Pull a page and serialize each record to JSON, preserving source order.
    async fn pull_json(&self, req: PullRequest) -> Result<Vec<Value>, ConnectorError>;
}

#[async_trait]
impl<C> ErasedSource for C
where
    C: Connector,
{
    async fn pull_json(&self, req: PullRequest) -> Result<Vec<Value>, ConnectorError> {
        let page = self.pull(req).await?;
        let mut out = Vec::with_capacity(page.records.len());
        for record in &page.records {
            out.push(serde_json::to_value(record)?);
        }
        Ok(out)
    }
}

/// A registered connector handle taking part in a federated query.
///
/// Carries the provenance metadata that every merged record is tagged with:
/// a stable [`ConnectorId`], a human-readable name, and a `kind` tag (e.g.
/// `"csv"`, `"http_json"`) used by the [`FederationPolicy`](super::FederationPolicy)
/// allow-lists.
#[derive(Clone)]
pub struct FederatedSource {
    id: ConnectorId,
    name: String,
    kind: String,
    inner: Arc<dyn ErasedSource>,
}

impl FederatedSource {
    /// Register `connector` under a stable id, display name, and kind tag.
    pub fn new<C>(
        id: impl Into<ConnectorId>,
        name: impl Into<String>,
        kind: impl Into<String>,
        connector: C,
    ) -> Self
    where
        C: Connector + 'static,
    {
        Self {
            id: id.into(),
            name: name.into(),
            kind: kind.into(),
            inner: Arc::new(connector),
        }
    }

    /// The stable connector id used for provenance tagging and allow-lists.
    pub fn id(&self) -> &ConnectorId {
        &self.id
    }

    /// The human-readable source name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The source kind tag (e.g. `"csv"`), matched by policy allow-lists.
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// Pull this source's records as JSON values in stable source order.
    pub(crate) async fn pull_json(&self, req: PullRequest) -> Result<Vec<Value>, ConnectorError> {
        self.inner.pull_json(req).await
    }
}

impl fmt::Debug for FederatedSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FederatedSource")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

impl From<&str> for ConnectorId {
    fn from(value: &str) -> Self {
        ConnectorId::new(value)
    }
}

impl From<String> for ConnectorId {
    fn from(value: String) -> Self {
        ConnectorId::new(value)
    }
}
