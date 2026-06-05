use convergio_server_core::ApiError;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone)]
pub(super) struct GatewayConfig {
    pub(super) allowlist: Option<BTreeMap<String, BTreeSet<String>>>,
    pub(super) token_caps: BTreeMap<String, u32>,
    pub(super) default_token_cap: u32,
    pub(super) azure_url: Option<String>,
    pub(super) anthropic_url: Option<String>,
    pub(super) mistral_url: Option<String>,
}

impl GatewayConfig {
    pub(super) fn from_env() -> Self {
        let allowlist = std::env::var("LLM_GATEWAY_PURPOSE_MODEL_ALLOWLIST_JSON")
            .ok()
            .and_then(|raw| parse_allowlist(&raw));

        let token_caps = std::env::var("LLM_GATEWAY_PURPOSE_TOKEN_CAP_JSON")
            .ok()
            .and_then(|raw| serde_json::from_str::<BTreeMap<String, u32>>(&raw).ok())
            .unwrap_or_default();

        let default_token_cap = std::env::var("LLM_GATEWAY_MAX_OUTPUT_TOKENS")
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(1024);

        let azure_url = std::env::var("LLM_GATEWAY_AZURE_OPENAI_URL").ok();
        let anthropic_url = std::env::var("LLM_GATEWAY_ANTHROPIC_URL").ok();
        let mistral_url = std::env::var("LLM_GATEWAY_MISTRAL_URL").ok();

        Self {
            allowlist,
            token_caps,
            default_token_cap,
            azure_url,
            anthropic_url,
            mistral_url,
        }
    }

    pub(super) fn cap_for_purpose(&self, purpose: &str) -> u32 {
        self.token_caps
            .get(purpose)
            .copied()
            .unwrap_or(self.default_token_cap)
    }

    pub(super) fn enforce_allowlist(&self, purpose: &str, model_id: &str) -> Result<(), ApiError> {
        let Some(allowlist) = &self.allowlist else {
            // MVP: allowlist is optional, but when configured it is enforced strictly.
            return Ok(());
        };
        let Some(models) = allowlist.get(purpose) else {
            return Err(ApiError::Validation {
                code: "purpose_not_allowed",
                message: format!("purpose '{purpose}' is not present in allowlist"),
            });
        };
        if !models.contains(model_id) {
            return Err(ApiError::Validation {
                code: "model_not_allowed",
                message: format!("model '{model_id}' is not allowed for purpose '{purpose}'"),
            });
        }
        Ok(())
    }
}

fn parse_allowlist(raw: &str) -> Option<BTreeMap<String, BTreeSet<String>>> {
    let map: BTreeMap<String, Vec<String>> = serde_json::from_str(raw).ok()?;
    Some(
        map.into_iter()
            .map(|(k, v)| (k, v.into_iter().collect::<BTreeSet<_>>()))
            .collect(),
    )
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ProviderChoice<'a> {
    pub(super) id: &'a str,
    pub(super) base_url: &'a str,
}

pub(super) fn choose_provider<'a>(
    cfg: &'a GatewayConfig,
    residency: Option<&str>,
) -> Result<ProviderChoice<'a>, ApiError> {
    let eu = residency
        .map(|s| s.eq_ignore_ascii_case("eu"))
        .unwrap_or(false);

    if eu {
        if let Some(url) = cfg.mistral_url.as_deref() {
            return Ok(ProviderChoice {
                id: "mistral",
                base_url: url,
            });
        }
    }

    if let Some(url) = cfg.azure_url.as_deref() {
        return Ok(ProviderChoice {
            id: "azure_openai",
            base_url: url,
        });
    }

    if let Some(url) = cfg.anthropic_url.as_deref() {
        return Ok(ProviderChoice {
            id: "anthropic",
            base_url: url,
        });
    }

    if let Some(url) = cfg.mistral_url.as_deref() {
        return Ok(ProviderChoice {
            id: "mistral",
            base_url: url,
        });
    }

    Err(ApiError::BadRequest {
        code: "no_providers_configured",
        message: "no LLM providers configured; set LLM_GATEWAY_*_URL env vars".into(),
    })
}

/// Extract `(input_tokens, output_tokens)` from the provider response when present.
/// Kept here so provider output stays opaque to the route handler.
pub(super) fn extract_usage_tokens(result: &Value) -> (Option<i64>, Option<i64>) {
    let usage = result.get("usage");
    let input = usage
        .and_then(|u| u.get("input_tokens"))
        .and_then(|v| v.as_i64());
    let output = usage
        .and_then(|u| u.get("output_tokens"))
        .and_then(|v| v.as_i64());
    (input, output)
}
