//! `cvg ontology diff|lineage|branch-diff` handlers (ADR-0060, W1 T9).
//!
//! Thin HTTP client wrappers — domain logic lives in
//! `convergio-ontology`; the daemon owns determinism and golden
//! posture, this module is only concerned with shaping the request
//! and printing the response.

use super::ontology_types::GraphFormatArg;
use super::{Client, OutputMode};
use anyhow::Result;

/// Handle `cvg ontology diff <name> --from N --to M [--format …]`.
pub async fn diff(
    client: &Client,
    output: OutputMode,
    name: &str,
    from: i64,
    to: i64,
    format: GraphFormatArg,
) -> Result<()> {
    let path = format!(
        "/v1/ontology/diff/object/{name}?from={from}&to={to}&format={}",
        format.as_query()
    );
    render(client, output, format, &path).await
}

/// Handle `cvg ontology lineage <name> [--format …]`.
pub async fn lineage(
    client: &Client,
    output: OutputMode,
    name: &str,
    format: GraphFormatArg,
) -> Result<()> {
    let path = format!(
        "/v1/ontology/lineage/object/{name}?format={}",
        format.as_query()
    );
    render(client, output, format, &path).await
}

/// Handle `cvg ontology branch-diff <name> [--format …]`.
///
/// The daemon returns HTTP 501 in W1 (branching itself ships with
/// ADR-0059). We translate that into a stable human-readable line on
/// the operator side so the command does not look broken.
pub async fn branch_diff(
    client: &Client,
    output: OutputMode,
    name: &str,
    format: GraphFormatArg,
) -> Result<()> {
    let path = format!(
        "/v1/ontology/branch-diff/object/{name}?format={}",
        format.as_query()
    );
    match client.get_bytes(&path).await {
        Ok(bytes) => write_bytes(&bytes),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("501") || msg.contains("not_implemented") {
                match output {
                    OutputMode::Json => println!(
                        "{{\"error\":{{\"code\":\"not_implemented\",\"message\":\"branch diff lands with ADR-0059\"}}}}"
                    ),
                    _ => println!(
                        "branch diff for `{name}` is not implemented yet (lands with ADR-0059)"
                    ),
                }
                Ok(())
            } else {
                Err(e)
            }
        }
    }
}

async fn render(
    client: &Client,
    output: OutputMode,
    format: GraphFormatArg,
    path: &str,
) -> Result<()> {
    let bytes = client.get_bytes(path).await?;
    match (format, output) {
        (GraphFormatArg::Json, OutputMode::Human) => {
            // Pretty-print the JSON body, otherwise byte-identical.
            let v: serde_json::Value = serde_json::from_slice(&bytes)?;
            println!("{}", serde_json::to_string_pretty(&v)?);
            Ok(())
        }
        _ => write_bytes(&bytes),
    }
}

fn write_bytes(bytes: &[u8]) -> Result<()> {
    use std::io::Write;
    std::io::stdout().write_all(bytes)?;
    Ok(())
}
