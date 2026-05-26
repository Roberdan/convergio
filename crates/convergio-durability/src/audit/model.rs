//! Audit log domain types — kept separate from the writer/verifier so
//! that callers (HTTP layer, CLI) can serialize them without pulling
//! in the DB pool.

use serde::{Deserialize, Serialize};

/// Logical entity affected by an event.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityKind {
    /// `plans` row.
    Plan,
    /// `tasks` row.
    Task,
    /// `evidence` row.
    Evidence,
    /// `agents` row.
    Agent,
    /// CRDT operation batch or materialized cell.
    Crdt,
    /// Workspace coordination resource, lease, or patch event.
    Workspace,
    /// Capability registry event.
    Capability,
    /// Ontology branch / entry.
    Ontology,
    /// Free-form entity reference for agent-emitted custom audit rows
    /// (ADR-0002 § Custom kinds, P2-2). The `entity_id` is opaque to
    /// the daemon — agents use it as a correlation key.
    Free,
}

impl EntityKind {
    /// String tag persisted in the DB.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Plan => "plan",
            Self::Task => "task",
            Self::Evidence => "evidence",
            Self::Agent => "agent",
            Self::Crdt => "crdt",
            Self::Workspace => "workspace",
            Self::Capability => "capability",
            Self::Ontology => "ontology",
            Self::Free => "free",
        }
    }
}

/// One persisted audit row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// UUID of this row.
    pub id: String,
    /// Monotonic sequence number (1-based).
    pub seq: i64,
    /// Logical entity affected.
    pub entity_type: String,
    /// Entity primary key.
    pub entity_id: String,
    /// Logical transition (e.g. `plan.created`, `task.in_progress`).
    pub transition: String,
    /// Canonical JSON payload that was hashed.
    pub payload: String,
    /// Agent responsible for this transition, if known.
    pub agent_id: Option<String>,
    /// Hash of the previous row.
    pub prev_hash: String,
    /// `sha256(prev_hash || payload)` as hex.
    pub hash: String,
    /// RFC3339 UTC timestamp.
    pub created_at: String,
}

/// Outcome of [`crate::audit::AuditLog::verify`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyReport {
    /// True iff every row in the requested range hashes correctly and
    /// links to the previous one.
    pub ok: bool,
    /// Number of rows checked.
    pub checked: i64,
    /// First sequence number where verification failed, or `None` if ok.
    pub broken_at: Option<i64>,
}
