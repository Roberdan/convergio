//! LLM Gateway (MVP).
//!
//! Internal-only endpoint providing:
//! - multi-provider routing (Azure OpenAI primary; Anthropic + Mistral fallback)
//! - per-purpose model allow-lists
//! - max token cap enforcement
//! - egress pre-flight: PII/secret redaction + prompt-injection flagging
//! - optional output-schema validation of the provider response
//! - response caching keyed by (prompt_hash, model_id, retrieval_set_hash)
//! - W3C-PROV-JSON provenance emitted even on cache hits

mod config;
mod egress;
mod prov;
mod redact;
mod schema;
mod util;

use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use chrono::Utc;
use convergio_server_core::{ApiError, AppState};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use config::{choose_provider, extract_usage_tokens, GatewayConfig};
use egress::EgressReport;
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
    /// Optional JSON Schema (subset) the provider response must satisfy.
    #[serde(default)]
    expected_output_schema: Option<Value>,
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
    /// W3C-PROV-JSON bundle emitted on every response (including cache hits).
    provenance: Value,
    /// Call metadata (cache status, provider, timing).
    meta: CallMeta,
    /// Egress pre-flight summary (redactions applied, injection flags).
    egress: EgressReport,
}

#[derive(Debug, Serialize)]
struct CallMeta {
    prompt_hash: String,
    model_id: String,
    retrieval_set_hash: String,
    purpose: String,
    provider_id: String,
    cache_hit: bool,
    cached_at: Option<String>,
    generated_at: String,
}

/// Returns true when injection-flagged prompts must be refused (opt-in).
fn block_injection_enabled() -> bool {
    std::env::var("LLM_GATEWAY_BLOCK_INJECTION")
        .map(|v| matches!(v.as_str(), "1" | "true" | "yes"))
        .unwrap_or(false)
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

    // Egress pre-flight: mask PII/secrets and flag injection before sending.
    let egress = egress::preflight(&req.prompt);
    if egress.report.injection_flagged && block_injection_enabled() {
        return Err(ApiError::BadRequest {
            code: "egress_injection_blocked",
            message: format!(
                "outbound prompt flagged for injection: {:?}",
                egress.report.injection_signals
            ),
        });
    }
    let safe_prompt = egress.safe_prompt;

    let prompt_hash = sha256_hex(safe_prompt.as_bytes());
    let retrieval_set_hash = req.retrieval_set_hash.clone();

    let mut cached_at = None;
    let (result, provider_id, cache_hit) = if let Some(hit) = read_cache(
        &state,
        req.cache,
        &prompt_hash,
        &req.model_id,
        &retrieval_set_hash,
    )
    .await?
    {
        cached_at = Some(hit.created_at.to_rfc3339());
        (hit.response, hit.provider_id, true)
    } else {
        let (result, provider_id) = call_provider(&cfg, &req, &safe_prompt, requested).await?;
        if req.cache {
            write_cache(
                &state,
                &result,
                &provider_id,
                &prompt_hash,
                &req.model_id,
                &retrieval_set_hash,
            )
            .await?;
        }
        (result, provider_id, false)
    };

    // Output-schema validation runs for cache hits too: the schema is a
    // per-request fence, not part of the cache key.
    if let Some(expected) = &req.expected_output_schema {
        schema::validate(expected, &result).map_err(|reason| ApiError::BadRequest {
            code: "output_schema_mismatch",
            message: format!("provider response failed output schema: {reason}"),
        })?;
    }

    let generated_at = Utc::now().to_rfc3339();
    let provenance = prov::build(&prov::ProvInputs {
        prompt_hash: &prompt_hash,
        model_id: &req.model_id,
        provider_id: &provider_id,
        cache_hit,
        generated_at: &generated_at,
    })?;

    let meta = CallMeta {
        prompt_hash,
        model_id: req.model_id,
        retrieval_set_hash,
        purpose: req.purpose,
        provider_id,
        cache_hit,
        cached_at,
        generated_at,
    };

    Ok(Json(CallResponse {
        result,
        provenance,
        meta,
        egress: egress.report,
    }))
}

async fn read_cache(
    state: &AppState,
    enabled: bool,
    prompt_hash: &str,
    model_id: &str,
    retrieval_set_hash: &str,
) -> Result<Option<convergio_durability::store::LlmGatewayCacheEntry>, ApiError> {
    if !enabled {
        return Ok(None);
    }
    state
        .durability
        .llm_gateway_cache()
        .get(prompt_hash, model_id, retrieval_set_hash)
        .await
        .map_err(|e| ApiError::Internal(format!("cache read failed: {e}")))
}

async fn write_cache(
    state: &AppState,
    result: &Value,
    provider_id: &str,
    prompt_hash: &str,
    model_id: &str,
    retrieval_set_hash: &str,
) -> Result<(), ApiError> {
    let (input_tokens, output_tokens) = extract_usage_tokens(result);
    let entry = convergio_durability::store::LlmGatewayCacheEntry {
        provider_id: provider_id.to_string(),
        response: result.clone(),
        input_tokens,
        output_tokens,
        created_at: Utc::now(),
    };
    state
        .durability
        .llm_gateway_cache()
        .put(prompt_hash, model_id, retrieval_set_hash, &entry)
        .await
        .map_err(|e| ApiError::Internal(format!("cache write failed: {e}")))
}

async fn call_provider(
    cfg: &GatewayConfig,
    req: &CallRequest,
    safe_prompt: &str,
    requested: u32,
) -> Result<(Value, String), ApiError> {
    let provider = choose_provider(cfg, req.residency.as_deref())?;
    let provider_id = provider.id.to_string();

    let client = reqwest::Client::new();
    let url = format!("{}/v1/complete", provider.base_url.trim_end_matches('/'));

    // Provider-normalized request body — carries the egress-safe prompt.
    let body = json!({
        "model_id": req.model_id,
        "prompt": safe_prompt,
        "max_output_tokens": requested,
        "purpose": req.purpose,
        "retrieval_set_hash": req.retrieval_set_hash,
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

    Ok((result, provider_id))
}
