//! Refusal gate for federated queries.
//!
//! A [`FederationPolicy`] declares the limits a federated query must respect.
//! It is evaluated **before** any connector runs; a violating query is refused
//! with [`ConnectorError::FederationRefused`] and no source is ever pulled.
//! This is the "refuse rather than silently truncate" posture applied to the
//! federation layer.

use super::query::FederatedQuery;
use crate::error::ConnectorError;
use std::collections::BTreeSet;

/// Declarative limits enforced by the gate before execution.
///
/// Every bound is opt-in: an unset (`None`) field imposes no constraint.
#[derive(Debug, Clone, Default)]
pub struct FederationPolicy {
    max_sources: Option<usize>,
    max_total_records: Option<usize>,
    max_per_source_limit: Option<u32>,
    allowed_ids: Option<BTreeSet<String>>,
    allowed_kinds: Option<BTreeSet<String>>,
}

impl FederationPolicy {
    /// A permissive policy with no limits.
    pub fn unrestricted() -> Self {
        Self::default()
    }

    /// Cap the number of sources a single query may fan out to.
    pub fn with_max_sources(mut self, max: usize) -> Self {
        self.max_sources = Some(max);
        self
    }

    /// Cap the overall result size the query may declare.
    pub fn with_max_total_records(mut self, max: usize) -> Self {
        self.max_total_records = Some(max);
        self
    }

    /// Cap the per-source page limit the query may request.
    pub fn with_max_per_source_limit(mut self, max: u32) -> Self {
        self.max_per_source_limit = Some(max);
        self
    }

    /// Restrict queries to the given set of source ids.
    pub fn with_allowed_ids(mut self, ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.allowed_ids = Some(ids.into_iter().map(Into::into).collect());
        self
    }

    /// Restrict queries to the given set of source kinds.
    pub fn with_allowed_kinds(
        mut self,
        kinds: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.allowed_kinds = Some(kinds.into_iter().map(Into::into).collect());
        self
    }

    /// Evaluate the policy against `query`.
    ///
    /// Returns `Ok(())` when the query is admissible, or
    /// [`ConnectorError::FederationRefused`] with a stable reason on the first
    /// violation found. No connector is contacted during evaluation.
    pub fn evaluate(&self, query: &FederatedQuery) -> Result<(), ConnectorError> {
        let sources = query.sources();

        if let Some(max) = self.max_sources {
            if sources.len() > max {
                return Err(ConnectorError::federation_refused(format!(
                    "too many sources: {} > max {max}",
                    sources.len()
                )));
            }
        }

        if let Some(allowed) = &self.allowed_ids {
            for source in sources {
                if !allowed.contains(source.id().as_str()) {
                    return Err(ConnectorError::federation_refused(format!(
                        "source id '{}' is not in the allow-list",
                        source.id().as_str()
                    )));
                }
            }
        }

        if let Some(allowed) = &self.allowed_kinds {
            for source in sources {
                if !allowed.contains(source.kind()) {
                    return Err(ConnectorError::federation_refused(format!(
                        "source kind '{}' is not allowed",
                        source.kind()
                    )));
                }
            }
        }

        if let Some(max) = self.max_per_source_limit {
            let requested = query.per_source_limit();
            if requested == 0 || requested > max {
                return Err(ConnectorError::federation_refused(format!(
                    "per-source limit {} exceeds policy max {max}",
                    describe_limit(requested)
                )));
            }
        }

        if let Some(max) = self.max_total_records {
            let cap = query.result_cap();
            if cap == 0 || cap > max {
                return Err(ConnectorError::federation_refused(format!(
                    "result cap {} exceeds policy max total records {max}",
                    describe_limit_usize(cap)
                )));
            }
        }

        Ok(())
    }
}

/// Render a `u32` limit, treating `0` as the explicit string `unbounded`.
fn describe_limit(value: u32) -> String {
    if value == 0 {
        "unbounded".to_string()
    } else {
        value.to_string()
    }
}

/// Render a `usize` limit, treating `0` as the explicit string `unbounded`.
fn describe_limit_usize(value: usize) -> String {
    if value == 0 {
        "unbounded".to_string()
    } else {
        value.to_string()
    }
}
