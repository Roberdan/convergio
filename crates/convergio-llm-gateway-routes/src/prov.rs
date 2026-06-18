//! W3C-PROV-JSON bundle construction for each gateway call.
//!
//! Wraps [`convergio_provenance`] so every response (including cache hits)
//! carries a standards-shaped provenance bundle: the gateway call activity,
//! the responsible agent (model id + prompt hash), and the response entity.

use convergio_provenance::{emit_bundle, to_prov_json, Activity, Agent, Entity};
use convergio_server_core::ApiError;
use serde_json::Value;

/// Inputs needed to build the provenance bundle for one call.
pub(super) struct ProvInputs<'a> {
    /// SHA-256 of the egress-safe prompt actually sent to the provider.
    pub(super) prompt_hash: &'a str,
    /// Requested model identifier.
    pub(super) model_id: &'a str,
    /// Provider that served the response.
    pub(super) provider_id: &'a str,
    /// Whether the response came from cache.
    pub(super) cache_hit: bool,
    /// RFC-3339 instant the response was emitted.
    pub(super) generated_at: &'a str,
}

/// Build a W3C-PROV-JSON bundle as a `serde_json::Value`.
pub(super) fn build(inputs: &ProvInputs<'_>) -> Result<Value, ApiError> {
    let now = chrono::Utc::now();
    let activity_id = format!(
        "cvg:llm-gateway:{}:{}",
        inputs.prompt_hash, inputs.generated_at
    );
    let activity = Activity {
        id: activity_id,
        kind: if inputs.cache_hit {
            "llm.gateway.call.cache_hit".into()
        } else {
            "llm.gateway.call".into()
        },
        started_at: now,
        ended_at: Some(now),
    };
    let agent = Agent {
        id: format!("cvg:model:{}", inputs.model_id),
        label: format!(
            "model={} prompt_hash={} provider={}",
            inputs.model_id, inputs.prompt_hash, inputs.provider_id
        ),
    };
    let entity = Entity {
        id: format!("cvg:llm-response:{}", inputs.prompt_hash),
        kind: "llm.response".into(),
    };

    let bundle = emit_bundle(activity, agent, entity)
        .map_err(|e| ApiError::Internal(format!("provenance build failed: {e}")))?;
    let bytes =
        to_prov_json(&bundle).map_err(|e| ApiError::Internal(format!("prov-json failed: {e}")))?;
    serde_json::from_slice(&bytes)
        .map_err(|e| ApiError::Internal(format!("prov-json decode failed: {e}")))
}
