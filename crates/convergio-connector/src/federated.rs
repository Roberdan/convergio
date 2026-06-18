//! Federated query path + refusal gate over multiple connectors (W4, ADR-0057).
//!
//! A [`FederatedExecutor`] runs a single logical pull ([`FederatedQuery`])
//! across N registered [`FederatedSource`]s and merges their pages into one
//! stable-ordered [`FederatedResult`]. Each merged record is tagged with the
//! provenance of its origin (source id, name, and registration order).
//!
//! ## Refusal gate
//!
//! Before any connector is contacted, the query is checked against a
//! [`FederationPolicy`]. A query that violates declared limits (too many
//! sources, a disallowed id/kind, an over-large per-source limit, or an
//! over-large/unbounded result cap) is **refused** with
//! [`ConnectorError::FederationRefused`](crate::ConnectorError::FederationRefused)
//! and no source is ever pulled. This mirrors the daemon's "refuse rather than
//! silently truncate" posture.
//!
//! ## Heterogeneous record types
//!
//! Connectors carry an associated `Record` type, so a single result set can
//! only be homogeneous if every source agrees on it. To stay generic over the
//! [`Connector`](crate::Connector) trait — including sources with *different*
//! record types — each record is serialized into a common form,
//! `serde_json::Value`, at the federation boundary (see [`FederatedSource`]).
//!
//! ## Determinism / ordering
//!
//! The merged result is ordered by `(source_order, source-local record
//! order)`: all records from the first registered source (in that source's own
//! deterministic pull order) come before any from the second, and so on. Caps
//! truncate by dropping the tail of this stable order, so a given query always
//! yields byte-identical results.

mod policy;
mod query;
mod source;

pub use policy::FederationPolicy;
pub use query::{FederatedQuery, Projection};
pub use source::FederatedSource;

use crate::connector::PullRequest;
use crate::error::ConnectorError;
use crate::types::ConnectorId;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One merged record tagged with the provenance of its origin source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergedRecord {
    /// Stable id of the source that produced this record.
    pub source_id: ConnectorId,
    /// Human-readable name of the producing source.
    pub source_name: String,
    /// Zero-based registration order of the source within the query.
    pub source_order: usize,
    /// The record payload (projected if the query declared a projection).
    pub record: Value,
}

/// Number of merged records contributed by a single source after caps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceCount {
    /// Stable id of the source.
    pub source_id: ConnectorId,
    /// Zero-based registration order of the source within the query.
    pub source_order: usize,
    /// Records contributed to the final result set.
    pub count: usize,
}

/// The outcome of a federated execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FederatedResult {
    /// Merged records in stable `(source_order, source-local order)` order.
    pub records: Vec<MergedRecord>,
    /// Whether the overall result cap dropped one or more records.
    pub truncated: bool,
    /// Per-source contribution counts, in source registration order.
    pub counts: Vec<SourceCount>,
}

/// Executes [`FederatedQuery`]s under a [`FederationPolicy`].
#[derive(Debug, Clone, Default)]
pub struct FederatedExecutor {
    policy: FederationPolicy,
}

impl FederatedExecutor {
    /// Build an executor that enforces `policy` before every run.
    pub fn new(policy: FederationPolicy) -> Self {
        Self { policy }
    }

    /// Borrow the active policy.
    pub fn policy(&self) -> &FederationPolicy {
        &self.policy
    }

    /// Run the query: refuse-or-execute.
    ///
    /// The [`FederationPolicy`] is evaluated first; on violation the query is
    /// refused and **no** connector is pulled. Otherwise sources are pulled
    /// sequentially in registration order, merged in stable order, tagged with
    /// provenance, and truncated to the result cap.
    pub async fn execute(&self, query: &FederatedQuery) -> Result<FederatedResult, ConnectorError> {
        self.policy.evaluate(query)?;

        let per_source_limit = query.per_source_limit();
        let mut staged: Vec<MergedRecord> = Vec::new();

        for (source_order, source) in query.sources().iter().enumerate() {
            let req = PullRequest {
                stream: query.stream().map(str::to_string),
                since: None,
                limit: per_source_limit,
            };
            let mut pulled = source.pull_json(req).await?;
            // Defensively honour the per-source limit even if a connector
            // ignores the page-size hint, so caps are enforced uniformly.
            let limit = per_source_limit as usize;
            if limit != 0 && pulled.len() > limit {
                pulled.truncate(limit);
            }
            for value in pulled {
                let record = match query.projection() {
                    Some(projection) => projection.apply(value),
                    None => value,
                };
                staged.push(MergedRecord {
                    source_id: source.id().clone(),
                    source_name: source.name().to_string(),
                    source_order,
                    record,
                });
            }
        }

        let cap = query.result_cap();
        let truncated = cap != 0 && staged.len() > cap;
        if truncated {
            staged.truncate(cap);
        }

        let counts = tally(query, &staged);
        Ok(FederatedResult {
            records: staged,
            truncated,
            counts,
        })
    }
}

/// Compute per-source contribution counts from the final merged set.
fn tally(query: &FederatedQuery, records: &[MergedRecord]) -> Vec<SourceCount> {
    query
        .sources()
        .iter()
        .enumerate()
        .map(|(source_order, source)| {
            let count = records
                .iter()
                .filter(|r| r.source_order == source_order)
                .count();
            SourceCount {
                source_id: source.id().clone(),
                source_order,
                count,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests;
