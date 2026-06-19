//! Request-body construction and prompt resolution for `cvg llm call`
//! (W5, ADR-0058). Split from `llm.rs` so the pure, unit-tested wire
//! logic stays small and isolated from the HTTP/render path (300-line
//! cap). `cvg` stays a thin HTTP client: it only shapes the JSON body
//! the daemon's `/v1/llm-gateway/call` endpoint expects.

use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::io::Read;
use std::path::Path;

/// Inputs needed to build the `/v1/llm-gateway/call` request body.
pub(crate) struct CallInput {
    /// Operator-declared processing purpose (required; ADR-0054).
    pub(crate) purpose: String,
    /// Allow-listed model identifier (required).
    pub(crate) model: String,
    /// Already-resolved prompt text (literal, file, or stdin).
    pub(crate) prompt: String,
    /// Requested maximum output tokens (subject to per-purpose cap).
    pub(crate) max_tokens: Option<u32>,
    /// Hash describing the retrieval set used to build the prompt.
    pub(crate) retrieval_set_hash: Option<String>,
    /// Optional JSON Schema the provider response must satisfy.
    pub(crate) expected_schema: Option<Value>,
}

/// Build the JSON request body for `POST /v1/llm-gateway/call`.
///
/// Required fields (`purpose`, `model_id`, `prompt`) are always present;
/// optional fields are only included when supplied so the daemon applies
/// its own defaults rather than seeing `null`.
pub(crate) fn build_call_body(input: &CallInput) -> Value {
    let mut map = serde_json::Map::new();
    map.insert("purpose".into(), Value::String(input.purpose.clone()));
    map.insert("model_id".into(), Value::String(input.model.clone()));
    map.insert("prompt".into(), Value::String(input.prompt.clone()));
    if let Some(max) = input.max_tokens {
        map.insert("max_output_tokens".into(), json!(max));
    }
    if let Some(hash) = &input.retrieval_set_hash {
        map.insert("retrieval_set_hash".into(), Value::String(hash.clone()));
    }
    if let Some(schema) = &input.expected_schema {
        map.insert("expected_output_schema".into(), schema.clone());
    }
    Value::Object(map)
}

/// Resolve the prompt text from the mutually-exclusive sources:
/// `--prompt-file <PATH>`, stdin (when `--prompt -`), or the literal
/// `--prompt <TEXT>`. Errors cleanly when no source is supplied.
pub(crate) fn resolve_prompt(
    prompt: Option<&str>,
    prompt_file: Option<&Path>,
    stdin: &mut impl Read,
) -> Result<String> {
    if let Some(path) = prompt_file {
        return std::fs::read_to_string(path)
            .with_context(|| format!("reading prompt file {}", path.display()));
    }
    match prompt {
        Some("-") => {
            let mut buf = String::new();
            stdin
                .read_to_string(&mut buf)
                .context("reading prompt from stdin")?;
            Ok(buf)
        }
        Some(text) => Ok(text.to_owned()),
        None => anyhow::bail!(
            "a prompt is required: pass --prompt <TEXT>, --prompt - (stdin), or --prompt-file <PATH>"
        ),
    }
}

/// Load and parse the optional `--expected-schema-file` into a JSON value.
pub(crate) fn load_expected_schema(path: Option<&Path>) -> Result<Option<Value>> {
    match path {
        None => Ok(None),
        Some(path) => {
            let raw = std::fs::read_to_string(path)
                .with_context(|| format!("reading schema file {}", path.display()))?;
            let value: Value = serde_json::from_str(&raw)
                .with_context(|| format!("parsing JSON schema {}", path.display()))?;
            Ok(Some(value))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> CallInput {
        CallInput {
            purpose: "demo".into(),
            model: "test-model".into(),
            prompt: "ping".into(),
            max_tokens: None,
            retrieval_set_hash: None,
            expected_schema: None,
        }
    }

    #[test]
    fn body_has_required_fields_only_by_default() {
        let body = build_call_body(&input());
        assert_eq!(body["purpose"], "demo");
        assert_eq!(body["model_id"], "test-model");
        assert_eq!(body["prompt"], "ping");
        let map = body.as_object().unwrap();
        assert_eq!(map.len(), 3, "no optional keys should leak: {map:?}");
        assert!(map.get("max_output_tokens").is_none());
        assert!(map.get("retrieval_set_hash").is_none());
        assert!(map.get("expected_output_schema").is_none());
    }

    #[test]
    fn body_includes_optional_fields_when_set() {
        let mut input = input();
        input.max_tokens = Some(128);
        input.retrieval_set_hash = Some("abc123".into());
        input.expected_schema = Some(json!({"type": "object"}));
        let body = build_call_body(&input);
        assert_eq!(body["max_output_tokens"], 128);
        assert_eq!(body["retrieval_set_hash"], "abc123");
        assert_eq!(body["expected_output_schema"], json!({"type": "object"}));
    }

    #[test]
    fn resolve_prompt_uses_literal_text() {
        let got = resolve_prompt(Some("hello"), None, &mut std::io::empty()).unwrap();
        assert_eq!(got, "hello");
    }

    #[test]
    fn resolve_prompt_reads_stdin_for_dash() {
        let mut stdin = std::io::Cursor::new(b"from stdin".to_vec());
        let got = resolve_prompt(Some("-"), None, &mut stdin).unwrap();
        assert_eq!(got, "from stdin");
    }

    #[test]
    fn resolve_prompt_reads_file_over_literal() {
        let dir = std::env::temp_dir().join(format!("cvg-llm-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("prompt.txt");
        std::fs::write(&path, "file prompt").unwrap();
        let got = resolve_prompt(Some("ignored"), Some(&path), &mut std::io::empty()).unwrap();
        assert_eq!(got, "file prompt");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn resolve_prompt_errors_without_source() {
        let err = resolve_prompt(None, None, &mut std::io::empty()).unwrap_err();
        assert!(err.to_string().contains("a prompt is required"));
    }

    #[test]
    fn load_expected_schema_is_none_when_absent() {
        assert!(load_expected_schema(None).unwrap().is_none());
    }

    #[test]
    fn load_expected_schema_parses_file() {
        let dir = std::env::temp_dir().join(format!("cvg-llm-schema-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("schema.json");
        std::fs::write(&path, r#"{"type":"string"}"#).unwrap();
        let schema = load_expected_schema(Some(&path)).unwrap().unwrap();
        assert_eq!(schema, json!({"type": "string"}));
        std::fs::remove_dir_all(&dir).ok();
    }
}
