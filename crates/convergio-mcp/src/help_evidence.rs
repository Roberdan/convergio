//! Evidence-schema help payloads for `convergio.help`.

use serde_json::{json, Value};

pub(crate) fn schema() -> Value {
    json!({
        "evidence_required": "each task lists required evidence kinds",
        "payload": "JSON object; include concise command/output facts, not huge logs",
        "exit_code": "0 for successful command evidence; omit when not applicable",
        "known_kinds": {
            "usage": {
                "description": "token/cost telemetry for a session (optional unless required)",
                "payload": {
                    "input_tokens": "integer",
                    "output_tokens": "integer",
                    "model": "string",
                    "cost_usd": "number|null"
                }
            }
        }
    })
}
