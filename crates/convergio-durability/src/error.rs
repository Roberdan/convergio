//! Errors for Layer 1.

use thiserror::Error;

/// All errors the durability layer can produce.
#[derive(Debug, Error)]
pub enum DurabilityError {
    /// A row that should exist does not.
    #[error("not found: {entity} id={id}")]
    NotFound {
        /// Logical entity name (`plan`, `task`, ...).
        entity: &'static str,
        /// Identifier the caller passed.
        id: String,
    },

    /// A gate refused the requested transition.
    #[error("gate refused: {gate}: {reason}")]
    GateRefused {
        /// Name of the refusing gate.
        gate: &'static str,
        /// Human-readable reason — included in API responses.
        reason: String,
    },

    /// Public callers may not transition a task to `done` — that state
    /// is reserved for [`Durability::complete_validated_tasks`], which
    /// is invoked only by Thor on a passing verdict. See
    /// CONSTITUTION §6 (clients propose, daemon disposes) and
    /// ADR-0011.
    #[error(
        "done is set only by validation (cvg validate); agent transitions may not target done"
    )]
    DoneNotByThor,

    /// A submitted-only operation observed a task in another state.
    #[error("expected task {id} in 'submitted', found '{actual}'")]
    NotSubmitted {
        /// Task id.
        id: String,
        /// Actual current status.
        actual: &'static str,
    },

    /// `retry_task` was called on a task that is not in `failed`.
    #[error("expected task {id} in 'failed', found '{actual}'")]
    NotFailed {
        /// Task id.
        id: String,
        /// Actual current status.
        actual: &'static str,
    },

    /// `close_task_post_hoc` was called with an empty / missing reason.
    /// ADR-0026 mandates a non-empty reason on every post-hoc close
    /// so the audit row carries enough provenance to triage.
    #[error("post-hoc close requires a non-empty reason")]
    PostHocReasonMissing,

    /// `close_task_post_hoc` was called with a reason that does not meet
    /// the provenance rules (ADR-0026; hardened by F62).
    #[error("invalid post-hoc close reason: {reason}")]
    PostHocReasonInvalid {
        /// Validation failure explanation.
        reason: String,
    },

    /// `close_task_post_hoc` was called on an already-`done` task.
    /// Idempotency guard: re-closing would write a duplicate audit
    /// row with a contradictory `from` value.
    #[error("task {id} is already done; post-hoc close is a no-op")]
    AlreadyDone {
        /// Task id.
        id: String,
    },

    /// A done-only operation observed a task in another status.
    #[error("expected task {id} in 'done', found '{actual}'")]
    NotDone {
        /// Task id.
        id: String,
        /// Actual current status.
        actual: &'static str,
    },

    /// An agent re-registration was requested but the agent is not terminated.
    #[error("expected agent {id} in 'terminated', found '{actual}'")]
    AgentNotTerminated {
        /// Agent id.
        id: String,
        /// Actual agent status tag.
        actual: String,
    },

    /// `rename_plan` was called with an empty / blank title.
    #[error("plan title must be non-empty")]
    PlanTitleEmpty,

    /// Audit chain corruption detected.
    #[error("audit chain broken at seq={seq}")]
    AuditChainBroken {
        /// First sequence number where the chain breaks.
        seq: i64,
    },

    /// A CRDT operation reused an actor/counter pair with different data.
    #[error("crdt op conflict: actor={actor_id} counter={counter}")]
    CrdtOpConflict {
        /// Actor UUID.
        actor_id: String,
        /// Per-actor operation counter.
        counter: i64,
    },

    /// A cell contains operations that declare different CRDT types.
    #[error("mixed crdt types for {entity_type}/{entity_id}/{field_name}")]
    MixedCrdtTypes {
        /// Logical entity type.
        entity_type: String,
        /// Logical entity id.
        entity_id: String,
        /// Field name within the entity.
        field_name: String,
    },

    /// A CRDT operation type is not supported by the merge engine.
    #[error("unsupported crdt type: {crdt_type}")]
    UnsupportedCrdtType {
        /// Unsupported CRDT type.
        crdt_type: String,
    },

    /// A CRDT operation kind is not supported for the declared type.
    #[error("unsupported crdt operation: {crdt_type}/{op_kind}")]
    UnsupportedCrdtOperation {
        /// Declared CRDT type.
        crdt_type: String,
        /// Unsupported operation kind.
        op_kind: String,
    },

    /// A CRDT operation payload does not match the declared type.
    #[error("invalid crdt value for {crdt_type}: {reason}")]
    InvalidCrdtValue {
        /// Declared CRDT type.
        crdt_type: String,
        /// Validation failure reason.
        reason: String,
    },

    /// A workspace resource is already leased by another active agent.
    #[error("workspace lease conflict: resource={resource_id} lease={lease_id} agent={agent_id}")]
    WorkspaceLeaseConflict {
        /// Leased resource id.
        resource_id: String,
        /// Active conflicting lease id.
        lease_id: String,
        /// Agent holding the active lease.
        agent_id: String,
    },

    /// Workspace lease request is invalid.
    #[error("invalid workspace lease: {reason}")]
    InvalidWorkspaceLease {
        /// Validation failure reason.
        reason: String,
    },

    /// Agent registry input is invalid.
    #[error("invalid agent: {reason}")]
    InvalidAgent {
        /// Validation failure reason.
        reason: String,
    },

    /// Capability registry input is invalid.
    #[error("invalid capability: {reason}")]
    InvalidCapability {
        /// Validation failure reason.
        reason: String,
    },

    /// A patch proposal violates workspace coordination policy.
    #[error("workspace patch refused: {kind}: {reason}")]
    WorkspacePatchRefused {
        /// Conflict kind.
        kind: String,
        /// Human-readable refusal reason.
        reason: String,
    },

    /// Merge arbiter refused a queued patch proposal.
    #[error("workspace merge refused: {kind}: {reason}")]
    WorkspaceMergeRefused {
        /// Conflict kind.
        kind: String,
        /// Human-readable refusal reason.
        reason: String,
    },

    /// A plan transition was rejected because it does not follow the
    /// allowed lifecycle graph (`draft → active → completed|cancelled`,
    /// plus `draft → cancelled`).
    #[error("illegal plan transition: {from} → {to}")]
    IllegalPlanTransition {
        /// Current plan status as an opaque string tag.
        from: &'static str,
        /// Target plan status as an opaque string tag.
        to: &'static str,
    },

    /// Underlying database error.
    #[error(transparent)]
    Db(#[from] convergio_db::DbError),

    /// Sqlx error not surfaced via `convergio-db`.
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    /// JSON serialization / deserialization error.
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    /// Migration runner failure.
    #[error(transparent)]
    Migrate(#[from] sqlx::migrate::MigrateError),
}

/// Convenience alias.
pub type Result<T, E = DurabilityError> = std::result::Result<T, E>;
