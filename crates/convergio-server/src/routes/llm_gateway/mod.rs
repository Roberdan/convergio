//! LLM Gateway (MVP).
//!
//! Internal-only endpoint providing:
//! - multi-provider routing (Azure OpenAI primary; Anthropic + Mistral fallback)
//! - per-purpose model allow-lists
//! - max token cap enforcement
//! - response caching keyed by (prompt_hash, model_id, retrieval_set_hash)
//! - provenance emitted even on cache hits

mod config;
mod util;

use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use chrono::Utc;
use convergio_server_core::{ApiError, AppState};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use config::{choose_provider, extract_usage_tokens, GatewayConfig};
use util::sha256_hex;

/// Mount the LLM gateway routes.
pub fn router() -> Router<AppState> {
    Router::new().route("/v1/llm-gateway/call", post(call))
}

#[derive(Debug, Deserialize)]
struct CallRequest {
    /// Required: binds the request to an operator-declared purpose.
    purpose: String,
    /// Requested model identifier (opaque; allow-listed per purpose).
    model_id: String,
    /// Prompt text.
    prompt: String,
    /// Hash describing the retrieval set used to build the prompt.
    /// Use a stable value (e.g. a SHA-256 hex) and pass "none" when no retrieval.
    #[serde(default = "default_retrieval_set")]
    retrieval_set_hash: String,
    /// Residency hint: when set to "eu" the gateway prefers Mistral.
    #[serde(default)]
    residency: Option<String>,
    /// Requested maximum output tokens (subject to per-purpose cap).
    #[serde(default)]
    max_output_tokens: Option<u32>,
    /// When false, bypasses the cache.
    #[serde(default = "default_cache")]
    cache: bool,
}

fn default_retrieval_set() -> String {
    "none".into()
}

fn default_cache() -> bool {
    true
}

#[derive(Debug, Serialize)]
struct CallResponse {
    /// Provider-normalized response payload.
    result: Value,
    /// Provenance emitted on every response (including cache hits).
    provenance: Provenance,
}

#[derive(Debug, Serialize)]
struct Provenance {
    prompt_hash: String,
    model_id: String,
    retrieval_set_hash: String,
    purpose: String,
    provider_id: String,
    cache_hit: bool,
    cached_at: Option<String>,
    generated_at: String,
}

async fn call(
    State(state): State<AppState>,
    Json(req): Json<CallRequest>,
) -> Result<Json<CallResponse>, ApiError> {
    let cfg = GatewayConfig::from_env();

    cfg.enforce_allowlist(&req.purpose, &req.model_id)?;

    let cap = cfg.cap_for_purpose(&req.purpose);
    let requested = req.max_output_tokens.unwrap_or(cap);
    if requested > cap {
        return Err(ApiError::Validation {
            code: "token_cap_exceeded",
            message: format!(
                "requested max_output_tokens {requested} exceeds cap {cap} for purpose '{}'",
                req.purpose
            ),
        });
    }

    let prompt_hash = sha256_hex(req.prompt.as_bytes());
    let retrieval_set_hash = req.retrieval_set_hash;

    if req.cache {
        let cache = state.durability.llm_gateway_cache();
        if let Some(hit) = cache
            .get(&prompt_hash, &req.model_id, &retrieval_set_hash)
            .await
            .map_err(|e| ApiError::Internal(format!("cache read failed: {e}")))?
        {
            let provenance = Provenance {
                prompt_hash,
                model_id: req.model_id,
                retrieval_set_hash,
                purpose: req.purpose,
                provider_id: hit.provider_id,
                cache_hit: true,
                cached_at: Some(hit.created_at.to_rfc3339()),
                generated_at: Utc::now().to_rfc3339(),
            };
            return Ok(Json(CallResponse {
                result: hit.response,
                provenance,
            }));
        }
    }

    let provider = choose_provider(&cfg, req.residency.as_deref())?;
    let provider_id = provider.id.to_string();

    let client = reqwest::Client::new();
    let url = format!("{}/v1/complete", provider.base_url.trim_end_matches('/'));

    // Provider-normalized request body.
    let body = json!({
        "model_id": req.model_id,
        "prompt": req.prompt,
        "max_output_tokens": requested,
        "purpose": req.purpose,
        "retrieval_set_hash": retrieval_set_hash,
    });

    let resp = client
        .post(url)
        .json(&body)
        .send()
        .await
        .map_err(|e| ApiError::Internal(format!("provider request failed: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(ApiError::Internal(format!(
            "provider '{provider_id}' returned HTTP {status}: {text}"
        )));
    }

    let result: Value = resp
        .json()
        .await
        .map_err(|e| ApiError::Internal(format!("provider JSON decode failed: {e}")))?;

    if req.cache {
        let (input_tokens, output_tokens) = extract_usage_tokens(&result);
        let entry = convergio_durability::store::LlmGatewayCacheEntry {
            provider_id: provider_id.clone(),
            response: result.clone(),
            input_tokens,
            output_tokens,
            created_at: Utc::now(),
        };
        state
            .durability
            .llm_gateway_cache()
            .put(&prompt_hash, &req.model_id, &retrieval_set_hash, &entry)
            .await
            .map_err(|e| ApiError::Internal(format!("cache write failed: {e}")))?;
    }

    let provenance = Provenance {
        prompt_hash,
        model_id: req.model_id,
        retrieval_set_hash,
        purpose: req.purpose,
        provider_id,
        cache_hit: false,
        cached_at: None,
        generated_at: Utc::now().to_rfc3339(),
    };

    Ok(Json(CallResponse { result, provenance }))
}
