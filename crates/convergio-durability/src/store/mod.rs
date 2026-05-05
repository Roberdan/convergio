//! Data-access layer for plans, tasks, evidence, CRDT metadata and agents.
//!
//! Stores are thin: they handle the SQL and the (de)serialization, but
//! they don't enforce gates. Gates live in [`crate::gates`] and are run
//! by the [`crate::Durability`] facade before any state-changing call.

mod agent_queries;
mod agent_rows;
mod agent_summary;
mod agent_validation;
mod agents;
mod capabilities;
mod crdt;
mod crdt_merge;
mod crdt_merge_types;
mod crdt_rows;
mod evidence;
mod plan_pr_links;
mod plans;
mod tasks;
mod workspace;
mod workspace_merge;
mod workspace_merge_rows;
mod workspace_patch;
mod workspace_rows;

pub use agent_summary::{
    AgentAuditEntry, AgentLease, AgentPrLink, AgentSummary, CurrentTaskMeta, RetireStaleResult,
    StaleAgentReport,
};
pub use agents::{AgentHeartbeat, AgentRecord, AgentStore, NewAgent};
pub use capabilities::{Capability, CapabilityStore, NewCapability};
pub use crdt::{AppendOutcome, CrdtActor, CrdtOp, CrdtStore, NewCrdtOp};
pub use crdt_merge::CrdtCell;
pub use evidence::EvidenceStore;
pub use plan_pr_links::{NewPlanPrLink, PlanPrLinksStore};
pub use plans::PlanStore;
pub use tasks::TaskStore;
pub use workspace::{
    NewWorkspaceLease, NewWorkspaceResource, WorkspaceLease, WorkspaceResource, WorkspaceStore,
};
pub use workspace_merge::{MergeOutcome, MergeQueueItem};
pub use workspace_patch::{
    NewPatchProposal, PatchFile, PatchProposal, WorkspaceConflict, WorkspaceConflictRef,
};
