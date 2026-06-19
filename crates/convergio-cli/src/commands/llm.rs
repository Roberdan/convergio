//! `cvg llm` — call the daemon LLM gateway (W5, ADR-0058). `cvg` stays a
//! pure HTTP client: it shapes the request body and POSTs to
//! `/v1/llm-gateway/call`, honouring `--output human|json|plain`
//! (ADR-0043). The daemon owns provider routing, egress redaction,
//! schema validation, and provenance.

use super::llm_call::{build_call_body, load_expected_schema, resolve_prompt, CallInput};
use super::{Client, OutputMode};
use anyhow::Result;
use clap::Subcommand;
use serde_json::Value;
use std::path::PathBuf;

/// `cvg llm` subcommand surface.
#[derive(Subcommand)]
pub enum LlmCommand {
    /// Call the LLM gateway with a purpose-bound prompt.
    Call {
        /// Operator-declared processing purpose (required; ADR-0054).
        #[arg(long)]
        purpose: String,
        /// Allow-listed model identifier (required).
        #[arg(long)]
        model: String,
        /// Prompt text. Pass `-` to read the prompt from stdin.
        #[arg(long, conflicts_with = "prompt_file")]
        prompt: Option<String>,
        /// Read the prompt from a file instead of `--prompt`.
        #[arg(long, value_name = "PATH")]
        prompt_file: Option<PathBuf>,
        /// Requested maximum output tokens (subject to per-purpose cap).
        #[arg(long)]
        max_tokens: Option<u32>,
        /// Hash describing the retrieval set used to build the prompt.
        #[arg(long)]
        retrieval_set_hash: Option<String>,
        /// JSON Schema file the provider response must satisfy.
        #[arg(long, value_name = "PATH")]
        expected_schema_file: Option<PathBuf>,
    },
}

/// Dispatch a `cvg llm` subcommand.
pub async fn run(client: &Client, output: OutputMode, sub: LlmCommand) -> Result<()> {
    match sub {
        LlmCommand::Call {
            purpose,
            model,
            prompt,
            prompt_file,
            max_tokens,
            retrieval_set_hash,
            expected_schema_file,
        } => {
            let prompt = resolve_prompt(
                prompt.as_deref(),
                prompt_file.as_deref(),
                &mut std::io::stdin(),
            )?;
            let expected_schema = load_expected_schema(expected_schema_file.as_deref())?;
            let input = CallInput {
                purpose,
                model,
                prompt,
                max_tokens,
                retrieval_set_hash,
                expected_schema,
            };
            let body = build_call_body(&input);
            let resp: Value = client.post("/v1/llm-gateway/call", &body).await?;
            render(output, &resp)
        }
    }
}

/// Render the gateway response in the requested output mode.
fn render(output: OutputMode, resp: &Value) -> Result<()> {
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(resp)?),
        OutputMode::Plain => println!("{}", output_text(resp)),
        OutputMode::Human => {
            println!("{}", output_text(resp));
            let provider = resp["meta"]["provider_id"].as_str().unwrap_or("?");
            let cache_hit = resp["meta"]["cache_hit"].as_bool().unwrap_or(false);
            let injection = resp["egress"]["injection_flagged"]
                .as_bool()
                .unwrap_or(false);
            let mut summary = format!("— provider={provider} cache_hit={cache_hit}");
            if injection {
                summary.push_str(" injection_flagged=true");
            }
            println!("{summary}");
        }
    }
    Ok(())
}

/// Extract the model output text from the opaque provider `result`,
/// falling back to the compact JSON payload when no text field is found.
fn output_text(resp: &Value) -> String {
    let result = &resp["result"];
    for key in ["output_text", "text", "content", "completion"] {
        if let Some(text) = result.get(key).and_then(Value::as_str) {
            return text.to_owned();
        }
    }
    serde_json::to_string(result).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn output_text_prefers_known_keys() {
        let resp = json!({"result": {"output_text": "hi"}});
        assert_eq!(output_text(&resp), "hi");
        let resp = json!({"result": {"text": "yo"}});
        assert_eq!(output_text(&resp), "yo");
    }

    #[test]
    fn output_text_falls_back_to_json() {
        let resp = json!({"result": {"choices": [1]}});
        assert_eq!(output_text(&resp), r#"{"choices":[1]}"#);
    }
}
