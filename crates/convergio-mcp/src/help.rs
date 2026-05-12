//! Compact agent-facing help for the MCP bridge.

use convergio_api::{actions_json, Action, ActionCatalog, HelpRequest, HelpTopic, SCHEMA_VERSION};
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
        HelpTopic::Actions => match serde_json::from_str(actions_json()) {
            Ok(v) => v,
            Err(e) => json!({
                "schema_version": SCHEMA_VERSION,
                "error": format!("failed to parse generated actions.json: {e}"),
                "actions": [],
            }),
        },
        HelpTopic::Action => action_help(request.action),
        HelpTopic::EvidenceSchema => json!({
            "evidence_required": "each task lists required evidence kinds",
            "payload": "JSON object; include concise command/output facts, not huge logs",
            "exit_code": "0 for successful command evidence; omit when not applicable",
            "kinds": {
                "usage": {
                    "payload": {
                        "input_tokens": "integer",
                        "output_tokens": "integer",
                        "model": "string",
                        "cost_usd": "number|null"
                    },
                    "note": "telemetry evidence; often posted by runner adapters"
                }
            }
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

    crate::help_actions::dispatch(action)
}
