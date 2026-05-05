//! `cvg agent retire <id>` — retire a single agent by id.
//!
//! Closes C4 + C8 from the 2026-05-04 retro: today retiring an agent
//! requires a raw `curl` POST to `/v1/agent-registry/agents/:id/retire`
//! or the bulk `cvg agent retire-stale --apply`. Sub-agents kept
//! falling back to curl, and one set `status=idle` instead of
//! `retired` because the heartbeat route refuses `retired`.
//!
//! This handler turns the explicit single-agent retire into a
//! first-class CLI verb with localized output and the standard
//! human/json/plain rendering modes.

use super::{Client, OutputMode};
use anyhow::Result;
use convergio_i18n::Bundle;
use serde::Deserialize;
use serde_json::Value;

/// Minimal projection of the daemon's `AgentRecord` response — we
/// only need the fields used by the human/plain renderers; full JSON
/// is preserved separately for `--output json`.
#[derive(Debug, Deserialize)]
struct Retired {
    id: String,
    status: String,
}

/// Entry point for `cvg agent retire <id>`.
pub async fn run(client: &Client, bundle: &Bundle, output: OutputMode, id: &str) -> Result<()> {
    let raw: Value = match client
        .post::<_, Value>(
            &format!("/v1/agent-registry/agents/{id}/retire"),
            &serde_json::json!({}),
        )
        .await
    {
        Ok(v) => v,
        Err(e) => {
            // Surface the localized "not found" hint up-front so the
            // operator does not have to parse the raw HTTP error body.
            // Any other failure (network, 5xx) still propagates via the
            // returned anyhow chain.
            let msg = format!("{e}");
            if msg.contains("HTTP 404") {
                eprintln!("{}", bundle.t("agent-retire-not-found", &[("id", id)]));
            }
            return Err(e);
        }
    };
    let parsed: Retired = serde_json::from_value(raw.clone())?;
    match output {
        OutputMode::Human => println!(
            "{}",
            bundle.t("agent-retire-success", &[("id", parsed.id.as_str())])
        ),
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&raw)?),
        OutputMode::Plain => println!("{}\t{}", parsed.id, parsed.status),
    }
    Ok(())
}
