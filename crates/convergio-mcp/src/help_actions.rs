//! Per-capability help payloads for `action_help`.
//!
//! Extracted from `help.rs` (audit finding L6) so the central
//! `action_help` dispatcher stays small and so each capability owns
//! the schema strings its actions advertise. Keep these payloads in
//! sync with `convergio_api::Action` and the corresponding handler
//! in `actions.rs` / `bus_actions.rs`.

use convergio_api::Action;
use serde_json::{json, Value};

/// Returns the help payload for one action, or `None` when the action
/// is not in the family this helper covers.
type FamilyHelp = fn(Action) -> Option<Value>;

const FAMILIES: &[FamilyHelp] = &[
    task_lifecycle,
    bus,
    audit,
    crdt,
    agent_registry,
    capability,
    workspace,
    ontology,
    crate::help_actions_llm::llm_gateway,
];

/// Resolve a single action's help body by walking each capability family.
pub(crate) fn dispatch(action: Action) -> Value {
    for family in FAMILIES {
        if let Some(value) = family(action) {
            return value;
        }
    }
    // Every variant of `Action` is covered by exactly one family above;
    // this branch is unreachable in practice but keeps the function
    // total without panicking on a future enum variant.
    json!({"params": {}})
}

fn task_lifecycle(action: Action) -> Option<Value> {
    Some(match action {
        Action::Status => json!({"params": {}}),
        Action::CreatePlan => json!({
            "params": {
                "title": "string",
                "description": "string?",
                "project": "string?"
            }
        }),
        Action::CreateTask => json!({
            "params": {
                "plan_id": "uuid",
                "title": "string",
                "description": "string?",
                "wave": "integer?",
                "sequence": "integer?",
                "evidence_required": ["code", "test", "doc"]
            }
        }),
        Action::ListTasks | Action::NextTask => json!({"params": {"plan_id": "uuid"}}),
        Action::ClaimTask | Action::SubmitTask => json!({
            "params": {"task_id": "uuid", "agent_id": "string?"}
        }),
        Action::Heartbeat => json!({"params": {"task_id": "uuid"}}),
        Action::AddEvidence => json!({
            "params": {
                "task_id": "uuid",
                "kind": "code|test|doc|...",
                "payload": "object",
                "exit_code": "integer?"
            }
        }),
        Action::GetTaskContext => json!({
            "params": {
                "task_id": "uuid",
                "workspace_path": "path?",
                "message_topic": "string?",
                "message_cursor": "integer?",
                "message_limit": "integer?"
            }
        }),
        Action::ValidatePlan => json!({"params": {"plan_id": "uuid"}}),
        _ => return None,
    })
}

fn bus(action: Action) -> Option<Value> {
    Some(match action {
        Action::PublishMessage => json!({
            "params": {
                "plan_id": "uuid",
                "topic": "string",
                "sender": "agent-id?",
                "payload": "object"
            }
        }),
        Action::PollMessages => json!({
            "params": {
                "plan_id": "uuid",
                "topic": "string",
                "cursor": "integer?",
                "limit": "integer?"
            }
        }),
        Action::AckMessage => json!({
            "params": {
                "message_id": "uuid",
                "consumer": "agent-id?"
            }
        }),
        _ => return None,
    })
}

fn audit(action: Action) -> Option<Value> {
    Some(match action {
        Action::AuditVerify => json!({"params": {"from": "integer?", "to": "integer?"}}),
        Action::AuditAppend => json!({
            "_note": "Custom hash-chained audit row. kind must match \
                      ^[a-z][a-z0-9_]*\\.[a-z0-9_]+(\\.[a-z0-9_]+)*$ \
                      and must NOT start with daemon-reserved prefixes \
                      (task./plan./evidence./crdt./workspace./capability.) \
                      or use reserved names (agent.session_started, \
                      agent.retired, agent.retired_stale).",
            "params": {
                "kind": "myapp.session.pre_stop.check.1",
                "entity_kind": "agent | task | plan | evidence | free",
                "entity_id": "string (opaque correlation key)",
                "agent_id": "string?",
                "payload": "object"
            }
        }),
        Action::ExplainLastRefusal => json!({"params": {"task_id": "uuid?"}}),
        _ => return None,
    })
}

fn crdt(action: Action) -> Option<Value> {
    Some(match action {
        Action::ImportCrdtOps => json!({
            "params": {
                "agent_id": "string?",
                "ops": [{
                    "actor_id": "string",
                    "counter": "integer",
                    "entity_type": "task",
                    "entity_id": "string",
                    "field_name": "string",
                    "crdt_type": "lww_register|mv_register|or_set",
                    "op_kind": "set|add|remove",
                    "value": "json",
                    "hlc": "string"
                }]
            }
        }),
        Action::ListCrdtConflicts => json!({"params": {}}),
        _ => return None,
    })
}

fn agent_registry(action: Action) -> Option<Value> {
    Some(match action {
        Action::RegisterAgent => json!({
            "_note": "All agent actions use 'id' for the agent's own primary key \
                      (ADR-0043): register_agent, heartbeat_agent, and retire_agent \
                      all accept 'id'. This is the entity-self convention.",
            "params": {
                "id": "stable-agent-id (you choose; lower-case, no whitespace)",
                "kind": "claude | copilot | cursor | codex | shell | aider | claude-sdk | gpt-4o | ... (lower-case ASCII + - . _; max 64 chars)",
                "name": "string?",
                "host": "string?",
                "capabilities": ["code", "test"],
                "metadata": "object?"
            }
        }),
        Action::ListAgents => json!({"params": {}}),
        Action::HeartbeatAgent => json!({
            "_note": "Pass 'id' (the agent's own primary key, same convention as \
                      register_agent — ADR-0043). 'agent_id' is a deprecated alias \
                      accepted until 0.4.0 with a warning.",
            "params": {
                "id": "stable-agent-id (must already be registered)",
                "current_task_id": "uuid?",
                "status": "idle|working|unhealthy?"
            }
        }),
        Action::RetireAgent => json!({
            "_note": "Pass 'id' (the agent's own primary key — ADR-0043). \
                      'agent_id' is accepted as a deprecated alias until 0.4.0.",
            "params": {
                "id": "stable-agent-id (must already be registered)"
            }
        }),
        Action::SpawnRunner => json!({
            "_note": "kind is one of: shell | claude | copilot. All dispatch \
                      through the same local supervisor; the kind label is \
                      informational. Per ADR-0028 non-shell kinds point \
                      command at ~/.convergio/adapters/<kind>/run.sh, but \
                      any local executable is accepted.",
            "params": {
                "agent_id": "stable-agent-id",
                "kind": "shell | claude | copilot",
                "command": "/bin/sh",
                "args": ["-c", "echo hello"],
                "env": [["KEY", "VALUE"]],
                "plan_id": "uuid?",
                "task_id": "uuid?",
                "capabilities": ["shell"]
            }
        }),
        _ => return None,
    })
}

fn capability(action: Action) -> Option<Value> {
    Some(match action {
        Action::PlannerSolve => json!({
            "params": {
                "mission": "string"
            }
        }),
        Action::ListCapabilities => json!({"params": {}}),
        Action::GetCapability => json!({"params": {"name": "planner"}}),
        Action::AgentPrompt => json!({"params": {}}),
        _ => return None,
    })
}

fn workspace(action: Action) -> Option<Value> {
    Some(match action {
        Action::ClaimWorkspaceLease => json!({
            "params": {
                "resource": {
                    "kind": "file|directory|symbol|artifact|ci_lane",
                    "project": "string?",
                    "path": "string",
                    "symbol": "string?"
                },
                "task_id": "uuid?",
                "agent_id": "string",
                "purpose": "string?",
                "expires_at": "RFC3339 timestamp"
            }
        }),
        Action::ListWorkspaceLeases => json!({"params": {}}),
        Action::ReleaseWorkspaceLease => json!({"params": {"lease_id": "uuid"}}),
        Action::SubmitPatchProposal => json!({
            "params": {
                "task_id": "uuid",
                "agent_id": "string",
                "base_revision": "git sha",
                "patch": "unified diff",
                "files": [{
                    "path": "relative/path",
                    "project": "string?",
                    "base_hash": "sha256",
                    "current_hash": "sha256",
                    "proposed_hash": "sha256"
                }]
            }
        }),
        Action::EnqueuePatchProposal => json!({"params": {"proposal_id": "uuid"}}),
        Action::ProcessMergeQueue => json!({"params": {}}),
        Action::ListMergeQueue => json!({"params": {}}),
        Action::ListWorkspaceConflicts => json!({"params": {}}),
        _ => return None,
    })
}

fn ontology(action: Action) -> Option<Value> {
    Some(match action {
        Action::OntologyList => json!({"params": {}}),
        Action::OntologyDescribe => json!({
            "params": {
                "kind": "object | link",
                "name": "string"
            }
        }),
        Action::OntologyExport => json!({
            "params": {
                "name": "string",
                "format": "jsonschema | shacl",
                "version": "integer?"
            }
        }),
        _ => return None,
    })
}
