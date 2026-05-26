//! # convergio-durability — Layer 1
//!
//! Durability core for Convergio: persistent state for plans, tasks,
//! evidence and agents, plus a hash-chained audit log and a
//! server-enforced gate pipeline.
//!
//! This crate is **the** product. Everything above (`bus`, `lifecycle`,
//! reference implementation) builds on top. A user adopting only Layer 1
//! gets:
//!
//! 1. CRUD for plans / tasks / evidence
//! 2. State transitions vetted by the gate pipeline
//! 3. Hash-chained audit trail verifiable from any external process
//!
//! See [`AuditLog::verify`] for the verification API and
//! [ADR-0002](../../docs/adr/0002-audit-hash-chain.md) for the hashing
//! scheme.
//!
//! ## Module map
//!
//! - [`audit`] — append-only hash-chained log
//! - [`store`] — DAOs for `plans`, `tasks`, `evidence`, `agents`
//! - [`gates`] — gate pipeline (plan_status, evidence, crdt_conflict,
//!   no_debt, no_stub, wire_check, no_secrets, zero_warnings,
//!   wave_sequence, pr_link)
//! - [`model`] — domain types
//!
//! ## Quickstart
//!
//! ```no_run
//! use convergio_db::Pool;
//! use convergio_durability::{init, Durability, NewPlan};
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let pool = Pool::connect("sqlite://./state.db").await?;
//! init(&pool).await?;                 // run migrations
//!
//! let dur = Durability::new(pool);
//! let plan = dur.create_plan(NewPlan {
//!     title: "first plan".into(),
//!     description: None,
//!     project: None,
//! }).await?;
//! println!("created plan {}", plan.id);
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]

pub mod audit;
pub mod capability_signature;
pub mod error;
pub mod gates;
pub mod model;
pub mod reaper;
pub mod store;
pub mod telemetry_collector;

mod agent_facade;
mod capability_facade;
mod crdt_facade;
mod facade;
mod facade_admin;
mod facade_evidence;
mod facade_plan_transitions;
mod facade_retry;
mod facade_telemetry;
mod facade_transitions;
mod migrate;
mod ontology_branch;
mod ontology_branch_store;
mod ontology_facade;
mod ontology_merge;
mod post_hoc_reason;
mod workspace_facade;

pub use capability_signature::{
    capability_signature_payload, verify_capability_signature, CapabilitySignatureRequest,
    CapabilitySignatureVerification, TrustedCapabilityKey,
};
pub use crdt_facade::CrdtImportResult;
pub use error::{DurabilityError, Result};
pub use facade::Durability;
pub use facade_telemetry::TelemetryCounters;
pub use migrate::init;
pub use model::{
    Evidence, NewPlan, NewTask, Plan, PlanStatus, RecentCompletedTask, Task, TaskStatus,
};
pub use ontology_branch::{
    OntologyBranch, OntologyBranchStatus, OntologyResolvedEntry, OntologyValueSource,
};
pub use ontology_branch_store::OntologyBranchStore;
pub use store::TelemetryPoint;
pub use store::{
    AgentAuditEntry, AgentHeartbeat, AgentLease, AgentPrLink, AgentRecord, AgentStore,
    AgentSummary, AppendOutcome, ClaimedTask, ClaimedTasks, CrdtActor, CrdtCell, CrdtOp, CrdtStore,
    CurrentTaskMeta, NewAgent, NewCrdtOp, RetireStaleResult, StaleAgentReport,
};
pub use store::{Capability, CapabilityStore, NewCapability};
pub use store::{EvalOutcome, EvalOutcomeStore, NewEvalOutcome, TaxonomyStore};
pub use store::{MergeOutcome, MergeQueueItem};
pub use store::{
    NewPatchProposal, PatchFile, PatchProposal, WorkspaceConflict, WorkspaceConflictRef,
};
pub use store::{NewPlanPrLink, PlanPrLink, PlanPrLinksStore};
pub use store::{
    NewWorkspaceLease, NewWorkspaceResource, WorkspaceLease, WorkspaceResource, WorkspaceStore,
};
pub use store::{PlanObjective, PlanObjectiveStore};
pub use store::{WorktreeHolder, WorktreeStore};
