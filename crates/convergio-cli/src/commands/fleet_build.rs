//! `cvg fleet build` — parse and embed all enabled fleet repos (ADR-0038, F2-8).

use super::{Client, OutputMode};
use anyhow::Result;
use serde_json::{json, Value};

/// Entry point called from `fleet::run`.
pub async fn run(client: &Client, output: OutputMode, refresh_similarity: bool) -> Result<()> {
    let body = json!({ "refresh_similarity": refresh_similarity });
    let resp: Value = client.post("/v1/fleet/build", &body).await?;
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&resp)?),
        OutputMode::Plain => {
            println!(
                "processed={} skipped={} embedded={} edges={}",
                resp["repos_processed"].as_u64().unwrap_or(0),
                resp["repos_skipped"].as_u64().unwrap_or(0),
                resp["embed"]["embedded"].as_u64().unwrap_or(0),
                resp["similar_edges_written"].as_u64().unwrap_or(0),
            );
        }
        OutputMode::Human => {
            let processed = resp["repos_processed"].as_u64().unwrap_or(0);
            let embedded = resp["embed"]["embedded"].as_u64().unwrap_or(0);
            let skipped = resp["embed"]["skipped_unchanged"].as_u64().unwrap_or(0);
            let model = resp["model"].as_str().unwrap_or("?");
            println!("Fleet build complete ({model}). Repos: {processed}");
            println!("  Files embedded: {embedded}  skipped: {skipped}");
            if refresh_similarity {
                let edges = resp["similarity"]["duplicates"].as_u64().unwrap_or(0)
                    + resp["similarity"]["similar_to"].as_u64().unwrap_or(0);
                println!("  Similarity edges: {edges}");
            }
        }
    }
    Ok(())
}
