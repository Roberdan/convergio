//! Versioned action names accepted by `convergio.act`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Closed set of task-oriented actions accepted by `convergio.act`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// Diagnose daemon and integration readiness.
    Status,
    /// Create a plan.
    CreatePlan,
    /// Create a task under a plan.
    CreateTask,
    /// List tasks for a plan.
    ListTasks,
    /// Find the next task an agent should work on.
    NextTask,
    /// Claim a task as in progress.
    ClaimTask,
    /// Touch a task heartbeat.
    Heartbeat,
    /// Attach evidence to a task.
    AddEvidence,
    /// Generate a compact task-scoped context packet.
    GetTaskContext,
    /// Publish a plan-scoped bus message.
    PublishMessage,
    /// Poll unacknowledged plan-scoped bus messages.
    PollMessages,
    /// Acknowledge a plan-scoped bus message.
    AckMessage,
    /// Submit a task and run gates.
    SubmitTask,
    /// Validate a plan. Promotes every still-`submitted` task to
    /// `done` atomically when the verdict is Pass — see ADR-0011.
    /// `complete_task` was removed in schema v2 because agents may not
    /// self-promote to `done` (CONSTITUTION §6).
    ValidatePlan,
    /// Verify the audit hash chain.
    AuditVerify,
    /// Append a custom hash-chained audit row (P2-2, ADR-0002 § Custom kinds).
    AuditAppend,
    /// Import and materialize a CRDT operation batch.
    ImportCrdtOps,
    /// List unresolved CRDT conflicts.
    ListCrdtConflicts,
    /// Register or refresh an agent identity.
    RegisterAgent,
    /// List durable agent identities.
    ListAgents,
    /// Record an agent registry heartbeat.
    HeartbeatAgent,
    /// Retire an agent identity.
    RetireAgent,
    /// Spawn the local shell runner adapter.
    SpawnRunner,
    /// Solve a mission through the installed planner capability.
    #[serde(rename = "planner.solve")]
    PlannerSolve,
    /// List installed local capabilities.
    ListCapabilities,
    /// Get one installed local capability.
    GetCapability,
    /// Claim a workspace resource lease.
    ClaimWorkspaceLease,
    /// List active workspace leases.
    ListWorkspaceLeases,
    /// Release a workspace resource lease.
    ReleaseWorkspaceLease,
    /// Submit a workspace patch proposal.
    SubmitPatchProposal,
    /// Enqueue an accepted patch proposal for merge arbitration.
    EnqueuePatchProposal,
    /// Process the next pending merge queue item.
    ProcessMergeQueue,
    /// List merge queue items.
    ListMergeQueue,
    /// List open workspace conflicts.
    ListWorkspaceConflicts,
    /// Explain the most recent gate refusal for a task.
    ExplainLastRefusal,
    /// Return the canonical prompt addendum for agents.
    AgentPrompt,
    /// Create a fleet plan (cross-repo, ADR-0038 F3-2).
    FleetPlanCreate,
    /// Show a fleet plan with its per-repo links.
    FleetPlanShow,
    /// Run the gate pipeline across every linked repo (F3-3, read-only).
    FleetPlanValidate,
    /// List ontology types (ObjectType + LinkType) in the registry (ADR-0053).
    #[serde(rename = "ontology.list")]
    OntologyList,
    /// Describe one ontology type with its inline properties (ADR-0053).
    #[serde(rename = "ontology.describe")]
    OntologyDescribe,
    /// Export an ontology ObjectType as byte-identical JSON-Schema or
    /// SHACL/JSON-LD (ADR-0053).
    #[serde(rename = "ontology.export")]
    OntologyExport,
}

impl Action {
    /// Every supported action in stable display order.
    pub const ALL: &'static [Self] = &[
        Self::Status,
        Self::CreatePlan,
        Self::CreateTask,
        Self::ListTasks,
        Self::NextTask,
        Self::ClaimTask,
        Self::Heartbeat,
        Self::AddEvidence,
        Self::GetTaskContext,
        Self::PublishMessage,
        Self::PollMessages,
        Self::AckMessage,
        Self::SubmitTask,
        Self::ValidatePlan,
        Self::AuditVerify,
        Self::AuditAppend,
        Self::ImportCrdtOps,
        Self::ListCrdtConflicts,
        Self::RegisterAgent,
        Self::ListAgents,
        Self::HeartbeatAgent,
        Self::RetireAgent,
        Self::SpawnRunner,
        Self::PlannerSolve,
        Self::ListCapabilities,
        Self::GetCapability,
        Self::ClaimWorkspaceLease,
        Self::ListWorkspaceLeases,
        Self::ReleaseWorkspaceLease,
        Self::SubmitPatchProposal,
        Self::EnqueuePatchProposal,
        Self::ProcessMergeQueue,
        Self::ListMergeQueue,
        Self::ListWorkspaceConflicts,
        Self::ExplainLastRefusal,
        Self::AgentPrompt,
        Self::FleetPlanCreate,
        Self::FleetPlanShow,
        Self::FleetPlanValidate,
        Self::OntologyList,
        Self::OntologyDescribe,
        Self::OntologyExport,
    ];

    /// Stable snake_case action name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Status => "status",
            Self::CreatePlan => "create_plan",
            Self::CreateTask => "create_task",
            Self::ListTasks => "list_tasks",
            Self::NextTask => "next_task",
            Self::ClaimTask => "claim_task",
            Self::Heartbeat => "heartbeat",
            Self::AddEvidence => "add_evidence",
            Self::GetTaskContext => "get_task_context",
            Self::PublishMessage => "publish_message",
            Self::PollMessages => "poll_messages",
            Self::AckMessage => "ack_message",
            Self::SubmitTask => "submit_task",
            Self::ValidatePlan => "validate_plan",
            Self::AuditVerify => "audit_verify",
            Self::AuditAppend => "audit_append",
            Self::ImportCrdtOps => "import_crdt_ops",
            Self::ListCrdtConflicts => "list_crdt_conflicts",
            Self::RegisterAgent => "register_agent",
            Self::ListAgents => "list_agents",
            Self::HeartbeatAgent => "heartbeat_agent",
            Self::RetireAgent => "retire_agent",
            Self::SpawnRunner => "spawn_runner",
            Self::PlannerSolve => "planner.solve",
            Self::ListCapabilities => "list_capabilities",
            Self::GetCapability => "get_capability",
            Self::ClaimWorkspaceLease => "claim_workspace_lease",
            Self::ListWorkspaceLeases => "list_workspace_leases",
            Self::ReleaseWorkspaceLease => "release_workspace_lease",
            Self::SubmitPatchProposal => "submit_patch_proposal",
            Self::EnqueuePatchProposal => "enqueue_patch_proposal",
            Self::ProcessMergeQueue => "process_merge_queue",
            Self::ListMergeQueue => "list_merge_queue",
            Self::ListWorkspaceConflicts => "list_workspace_conflicts",
            Self::ExplainLastRefusal => "explain_last_refusal",
            Self::AgentPrompt => "agent_prompt",
            Self::FleetPlanCreate => "fleet_plan_create",
            Self::FleetPlanShow => "fleet_plan_show",
            Self::FleetPlanValidate => "fleet_plan_validate",
            Self::OntologyList => "ontology.list",
            Self::OntologyDescribe => "ontology.describe",
            Self::OntologyExport => "ontology.export",
        }
    }

    /// Capability bucket used by the P3-1 registry.
    pub fn capability(self) -> &'static str {
        match self {
            Self::Status | Self::AgentPrompt => "status",
            Self::CreatePlan | Self::ValidatePlan => "plans",
            Self::CreateTask
            | Self::ListTasks
            | Self::NextTask
            | Self::ClaimTask
            | Self::Heartbeat
            | Self::SubmitTask
            | Self::GetTaskContext => "tasks",
            Self::AddEvidence => "evidence",
            Self::AuditVerify | Self::AuditAppend | Self::ExplainLastRefusal => "audit",
            Self::RegisterAgent
            | Self::ListAgents
            | Self::HeartbeatAgent
            | Self::RetireAgent
            | Self::SpawnRunner => "agents",
            Self::ListCapabilities | Self::GetCapability | Self::PlannerSolve => "capabilities",
            Self::ImportCrdtOps | Self::ListCrdtConflicts => "crdt",
            Self::ClaimWorkspaceLease
            | Self::ListWorkspaceLeases
            | Self::ReleaseWorkspaceLease
            | Self::SubmitPatchProposal
            | Self::EnqueuePatchProposal
            | Self::ProcessMergeQueue
            | Self::ListMergeQueue
            | Self::ListWorkspaceConflicts => "workspace",
            Self::PublishMessage | Self::PollMessages | Self::AckMessage => "validation",
            Self::FleetPlanCreate | Self::FleetPlanShow | Self::FleetPlanValidate => "fleet",
            Self::OntologyList | Self::OntologyDescribe | Self::OntologyExport => "ontology",
        }
    }

    /// One-line summary used by the P3-1 registry.
    pub fn summary(self) -> &'static str {
        match self {
            Self::Status => "Diagnose daemon and integration readiness.",
            Self::CreatePlan => "Create a plan.",
            Self::CreateTask => "Create a task under a plan.",
            Self::ListTasks => "List tasks for a plan.",
            Self::NextTask => "Find the next task to work on.",
            Self::ClaimTask => "Claim a task as in progress.",
            Self::Heartbeat => "Touch a task heartbeat.",
            Self::AddEvidence => "Attach evidence to a task.",
            Self::GetTaskContext => "Generate a compact task-scoped context packet.",
            Self::PublishMessage => "Publish a plan-scoped bus message.",
            Self::PollMessages => "Poll unacknowledged plan-scoped bus messages.",
            Self::AckMessage => "Acknowledge a plan-scoped bus message.",
            Self::SubmitTask => "Submit a task and run gates.",
            Self::ValidatePlan => "Validate a plan; promote submitted tasks to done.",
            Self::AuditVerify => "Verify the audit hash chain.",
            Self::AuditAppend => "Append a custom hash-chained audit row.",
            Self::ImportCrdtOps => "Import a CRDT operation batch.",
            Self::ListCrdtConflicts => "List unresolved CRDT conflicts.",
            Self::RegisterAgent => "Register or refresh an agent identity.",
            Self::ListAgents => "List durable agent identities.",
            Self::HeartbeatAgent => "Record an agent registry heartbeat.",
            Self::RetireAgent => "Retire an agent identity.",
            Self::SpawnRunner => "Spawn the local shell runner adapter.",
            Self::PlannerSolve => "Solve a mission through the installed planner capability.",
            Self::ListCapabilities => "List installed local capabilities.",
            Self::GetCapability => "Get one installed local capability.",
            Self::ClaimWorkspaceLease => "Claim a workspace resource lease.",
            Self::ListWorkspaceLeases => "List active workspace leases.",
            Self::ReleaseWorkspaceLease => "Release a workspace resource lease.",
            Self::SubmitPatchProposal => "Submit a workspace patch proposal.",
            Self::EnqueuePatchProposal => "Enqueue an accepted patch proposal.",
            Self::ProcessMergeQueue => "Process the next pending merge queue item.",
            Self::ListMergeQueue => "List merge queue items.",
            Self::ListWorkspaceConflicts => "List open workspace conflicts.",
            Self::ExplainLastRefusal => "Explain the most recent gate refusal for a task.",
            Self::AgentPrompt => "Return the canonical agent prompt addendum.",
            Self::FleetPlanCreate => "Create a cross-repo fleet plan (ADR-0038 F3-2).",
            Self::FleetPlanShow => "Show a fleet plan with its per-repo links.",
            Self::FleetPlanValidate => {
                "Run the gate pipeline across every linked repo (read-only)."
            }
            Self::OntologyList => "List ontology types (ObjectType + LinkType) in the registry.",
            Self::OntologyDescribe => "Describe one ontology type with its inline properties.",
            Self::OntologyExport => {
                "Export an ontology ObjectType as JSON-Schema or SHACL/JSON-LD."
            }
        }
    }
}
