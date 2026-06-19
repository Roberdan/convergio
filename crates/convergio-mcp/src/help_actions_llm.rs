//! LLM gateway help payload for `action_help` (W5, ADR-0058).
//!
//! Split from `help_actions.rs` to keep that dispatcher under the
//! 300-line Rust cap. Mirrors the gateway request body advertised by
//! the `cvg llm call` CLI and the `llm.call` handler in `actions.rs`.

use convergio_api::Action;
use serde_json::{json, Value};

/// Help body for the `llm.call` action, or `None` for any other action.
pub(crate) fn llm_gateway(action: Action) -> Option<Value> {
    Some(match action {
        Action::LlmCall => json!({
            "params": {
                "purpose": "string (required; operator-declared, ADR-0054)",
                "model_id": "string (required; allow-listed per purpose)",
                "prompt": "string (required)",
                "retrieval_set_hash": "string?",
                "max_output_tokens": "integer?",
                "expected_output_schema": "object?"
            }
        }),
        _ => return None,
    })
}
