//! `cvg ontology import <file>` — register a self-contained ontology
//! draft (objects + properties + links) through the daemon. The file is
//! the `ontology.json` emitted by an authoring tool (ADR-0080); the CLI
//! stays a thin HTTP client and the daemon owns registration + gates.

use super::{Client, OutputMode};
use anyhow::{Context, Result};
use serde_json::Value;
use std::path::Path;

/// Read the draft JSON file and POST it to `/v1/ontology/import`.
pub async fn import(client: &Client, output: OutputMode, file: &Path) -> Result<()> {
    let raw = std::fs::read_to_string(file)
        .with_context(|| format!("reading ontology draft {}", file.display()))?;
    let body: Value =
        serde_json::from_str(&raw).with_context(|| format!("parsing {}", file.display()))?;
    let report: Value = client.post("/v1/ontology/import", &body).await?;

    let count = |k: &str| report.get(k).and_then(Value::as_u64).unwrap_or(0);
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&report)?),
        OutputMode::Plain => println!(
            "{}\t{}\t{}",
            count("objects"),
            count("properties"),
            count("links")
        ),
        OutputMode::Human => println!(
            "imported {} object(s), {} property(ies), {} link(s)",
            count("objects"),
            count("properties"),
            count("links")
        ),
    }
    Ok(())
}
