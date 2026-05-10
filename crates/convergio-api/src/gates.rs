//! Gate precondition catalog schema (P3-2 — Palantir-inspired).
//!
//! This module defines the stable, serializable shapes returned by
//! `GET /v1/gates/preconditions`. It intentionally contains *no* daemon
//! business logic.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Schema version for `GET /v1/gates/preconditions`.
///
/// This is intentionally **independent** from [`crate::SCHEMA_VERSION`]
/// (the MCP action schema). The precondition catalog can evolve without
/// forcing a breaking bump to the agent action contract.
pub const GATE_PRECONDITIONS_SCHEMA_VERSION: &str = "1";

/// Declarative gate precondition.
///
/// Fields are intentionally simple strings so non-Rust clients can
/// consume the catalog without matching the daemon's internal enums.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GatePrecondition {
    /// Stable gate name (`Gate::name()`), e.g. `"no_debt"`.
    pub gate: String,

    /// Evidence kinds this gate *reads* when present.
    ///
    /// Special value: `"*"` means "all evidence kinds".
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reads_evidence_kinds: Vec<String>,

    /// Whether the gate enforces `task.evidence_required` coverage.
    #[serde(default)]
    pub enforces_task_evidence_required: bool,

    /// Task statuses for which this gate is active.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub active_target_status: Vec<String>,

    /// Stable refusal reason codes the gate may emit.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refusal_reasons: Vec<String>,
}

/// Response envelope for the gate precondition catalog endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GatePreconditionsCatalog {
    /// Catalog schema version.
    pub schema_version: String,
    /// One entry per gate in the default pipeline.
    pub preconditions: Vec<GatePrecondition>,
}
