//! Compensating-action mapping for the agent action surface.
//!
//! Each variant of [`Action`] either has a clean inverse on the same
//! surface (P3-3 — Palantir-inspired compensating actions) or none.
//! Keeping this mapping in a sibling module — rather than alongside the
//! enum, its names, capabilities, and summaries in `action.rs` — keeps
//! every individual file well under the 300-line cap as we add more
//! actions over time.

use crate::action::Action;

impl Action {
    /// Compensating action that undoes this one's side effects when
    /// a clean inverse exists (P3-3 — Palantir-inspired compensating
    /// actions). Returns `None` for actions whose effect cannot be
    /// reversed by another action of the same surface.
    pub fn compensate(self) -> Option<Self> {
        match self {
            Self::RegisterAgent => Some(Self::RetireAgent),
            Self::RetireAgent => Some(Self::RegisterAgent),
            Self::ClaimWorkspaceLease => Some(Self::ReleaseWorkspaceLease),
            Self::Status
            | Self::CreatePlan
            | Self::CreateTask
            | Self::ListTasks
            | Self::NextTask
            | Self::ClaimTask
            | Self::Heartbeat
            | Self::AddEvidence
            | Self::GetTaskContext
            | Self::PublishMessage
            | Self::PollMessages
            | Self::AckMessage
            | Self::SubmitTask
            | Self::ValidatePlan
            | Self::AuditVerify
            | Self::ImportCrdtOps
            | Self::ListCrdtConflicts
            | Self::ListAgents
            | Self::HeartbeatAgent
            | Self::SpawnRunner
            | Self::PlannerSolve
            | Self::ListCapabilities
            | Self::GetCapability
            | Self::ListWorkspaceLeases
            | Self::ReleaseWorkspaceLease
            | Self::SubmitPatchProposal
            | Self::EnqueuePatchProposal
            | Self::ProcessMergeQueue
            | Self::ListMergeQueue
            | Self::ListWorkspaceConflicts
            | Self::ExplainLastRefusal
            | Self::AgentPrompt
            | Self::AuditAppend
            | Self::FleetPlanCreate
            | Self::FleetPlanShow
            | Self::FleetPlanValidate
            | Self::OntologyList
            | Self::OntologyDescribe
            | Self::OntologyExport
            | Self::LlmCall => None,
        }
    }

    /// True iff [`Self::compensate`] returns `Some`.
    pub fn is_reversible(self) -> bool {
        self.compensate().is_some()
    }
}
