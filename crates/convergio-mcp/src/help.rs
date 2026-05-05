//! Compact agent-facing help for the MCP bridge.

use convergio_api::{Action, ActionCatalog, HelpRequest, HelpTopic, SCHEMA_VERSION};
use serde_json::{json, Value};

pub(crate) fn response(request: &HelpRequest) -> Value {
    match request.topic {
        HelpTopic::Quickstart => json!({
            "schema_version": SCHEMA_VERSION,
            "tools": ActionCatalog::current().tools,
            "capabilities": {
                "streaming": true,
                "streams": [
                    {"path": "/v1/audit/stream", "event": "audit"},
                    {"path": "/v1/plans/:plan_id/messages/stream", "event": "bus"}
                ],
            },
            "protocol": [
                "call convergio.help once per session",
                "use convergio.act with schema_version and action",
                "never claim done unless validate_plan returns Pass — agents may submit but only Thor sets done (ADR-0011)",
                "on gate_refused, fix issue, add evidence, retry"
            ],
        }),
        HelpTopic::Actions => json!(ActionCatalog::current()),
        HelpTopic::Action => action_help(request.action),
        HelpTopic::EvidenceSchema => json!({
            "evidence_required": "each task lists required evidence kinds",
            "payload": "JSON object; include concise command/output facts, not huge logs",
            "exit_code": "0 for successful command evidence; omit when not applicable",
        }),
        HelpTopic::GateRefusal => json!({
            "flow": [
                "read code/message/data from gate_refused response",
                "fix the root cause",
                "attach new evidence",
                "retry submit_task",
            ],
            "next": "fix_add_evidence_retry_submit",
        }),
        HelpTopic::Setup => json!({
            "install": "scripts/install-local.sh",
            "setup": "cvg setup",
            "start": "convergio start",
            "doctor": "cvg doctor --json",
        }),
        HelpTopic::Prompt => agent_prompt(),
    }
}

pub(crate) fn agent_prompt() -> Value {
    json!({
        "prompt": "Use Convergio as the local source of truth. Call convergio.help once. Use convergio.act for task lifecycle and evidence. If a gate refuses work, fix the reason, attach new evidence, and retry submit_task. Do not tell the user work is done until validate_plan returns Pass — agents submit, the validator (Thor) is the only path to done (ADR-0011)."
    })
}

fn action_help(action: Option<Action>) -> Value {
    let Some(action) = action else {
        return json!({
            "error": "missing action",
            "example": {"topic": "action", "action": "submit_task"}
        });
    };

    match action {
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
        Action::ValidatePlan => json!({"params": {"plan_id": "uuid"}}),
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
        Action::RegisterAgent => json!({
            "_note": "register_agent uses 'id' (the new agent id you choose). \
                      heartbeat_agent and retire_agent use 'agent_id' (an \
                      already-registered agent id). Two distinct fields, two \
                      distinct purposes — do not interchange.",
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
            "_note": "heartbeat_agent uses 'agent_id' (existing agent). \
                      Do not pass 'id' — that's the register_agent param.",
            "params": {
                "agent_id": "stable-agent-id (must already be registered)",
                "current_task_id": "uuid?",
                "status": "idle|working|unhealthy?"
            }
        }),
        Action::RetireAgent => json!({
            "_note": "retire_agent uses 'agent_id' (existing agent), like \
                      heartbeat_agent. Not 'id' (that's register_agent).",
            "params": {
                "agent_id": "stable-agent-id (must already be registered)"
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
        Action::PlannerSolve => json!({
            "params": {
                "mission": "string"
            }
        }),
        Action::ListCapabilities => json!({"params": {}}),
        Action::GetCapability => json!({"params": {"name": "planner"}}),
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
        Action::ExplainLastRefusal => json!({"params": {"task_id": "uuid?"}}),
        Action::AgentPrompt => json!({"params": {}}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use convergio_api::HelpVerbosity;

    fn action_request(action: Action) -> HelpRequest {
        HelpRequest {
            topic: HelpTopic::Action,
            action: Some(action),
            verbosity: HelpVerbosity::Short,
        }
    }

    #[test]
    fn action_help_covers_every_catalog_action() {
        for action in Action::ALL {
            let help = response(&action_request(*action));
            assert!(
                help.get("error").is_none(),
                "{} lacks action help: {help}",
                action.as_str()
            );
            assert!(
                help.get("params").is_some(),
                "{} action help lacks params: {help}",
                action.as_str()
            );
        }
    }

    #[test]
    fn actions_topic_matches_api_catalog() {
        let help = response(&HelpRequest {
            topic: HelpTopic::Actions,
            action: None,
            verbosity: HelpVerbosity::Short,
        });
        let actions = help.get("actions").and_then(Value::as_array).unwrap();
        assert_eq!(actions.len(), Action::ALL.len());
        for action in Action::ALL {
            assert!(actions.iter().any(|value| value == action.as_str()));
        }
    }
}
