//! The federated query description: which sources to fan out to and the caps.

use super::source::FederatedSource;
use serde_json::{Map, Value};

/// Optional field selection applied to every merged record.
///
/// When set, each record object is reduced to only the named top-level
/// fields (preserving their values); missing fields are dropped silently and
/// non-object records pass through unchanged.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Projection {
    fields: Vec<String>,
}

impl Projection {
    /// Build a projection that keeps only `fields`.
    pub fn new(fields: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            fields: fields.into_iter().map(Into::into).collect(),
        }
    }

    /// Whether this projection selects no fields (a no-op pass-through).
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// The selected field names.
    pub fn fields(&self) -> &[String] {
        &self.fields
    }

    /// Apply the projection to a single record value.
    pub(crate) fn apply(&self, value: Value) -> Value {
        if self.fields.is_empty() {
            return value;
        }
        match value {
            Value::Object(map) => {
                let mut out = Map::new();
                for field in &self.fields {
                    if let Some(v) = map.get(field) {
                        out.insert(field.clone(), v.clone());
                    }
                }
                Value::Object(out)
            }
            other => other,
        }
    }
}

/// A federated fan-out query across registered connector [`FederatedSource`]s.
///
/// `per_source_limit` is the page size requested from (and defensively
/// enforced on) each source; `result_cap` bounds the merged result set.
/// A value of `0` for either means "unbounded" — note that an unbounded cap
/// can itself be refused by a [`FederationPolicy`](super::FederationPolicy)
/// that declares a maximum total.
#[derive(Debug, Clone)]
pub struct FederatedQuery {
    sources: Vec<FederatedSource>,
    per_source_limit: u32,
    result_cap: usize,
    projection: Option<Projection>,
    stream: Option<String>,
}

impl FederatedQuery {
    /// Start a query over `sources` with unbounded caps and no projection.
    pub fn new(sources: impl IntoIterator<Item = FederatedSource>) -> Self {
        Self {
            sources: sources.into_iter().collect(),
            per_source_limit: 0,
            result_cap: 0,
            projection: None,
            stream: None,
        }
    }

    /// Set the per-source page limit (`0` = unbounded).
    pub fn with_per_source_limit(mut self, limit: u32) -> Self {
        self.per_source_limit = limit;
        self
    }

    /// Set the overall merged-result cap (`0` = unbounded).
    pub fn with_result_cap(mut self, cap: usize) -> Self {
        self.result_cap = cap;
        self
    }

    /// Restrict every merged record to the given projection.
    pub fn with_projection(mut self, projection: Projection) -> Self {
        self.projection = Some(projection);
        self
    }

    /// Pull from a specific named stream on every source.
    pub fn with_stream(mut self, stream: impl Into<String>) -> Self {
        self.stream = Some(stream.into());
        self
    }

    /// The registered sources, in fan-out (and result) order.
    pub fn sources(&self) -> &[FederatedSource] {
        &self.sources
    }

    /// The per-source page limit (`0` = unbounded).
    pub fn per_source_limit(&self) -> u32 {
        self.per_source_limit
    }

    /// The overall merged-result cap (`0` = unbounded).
    pub fn result_cap(&self) -> usize {
        self.result_cap
    }

    /// The active projection, if any.
    pub fn projection(&self) -> Option<&Projection> {
        self.projection.as_ref()
    }

    /// The target stream, if pinned.
    pub fn stream(&self) -> Option<&str> {
        self.stream.as_deref()
    }
}
