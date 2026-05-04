//! `cvg fleet patterns` — cross-repo cluster detection (ADR-0038, F2-9).

use super::{Client, OutputMode};
use anyhow::Result;
use serde_json::Value;

/// Entry point called from `fleet::run`.
pub async fn run(client: &Client, output: OutputMode, min_repos: usize) -> Result<()> {
    let resp: Value = client
        .get(&format!("/v1/fleet/patterns?min_repos={min_repos}"))
        .await?;
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&resp)?),
        OutputMode::Plain => print_plain(&resp),
        OutputMode::Human => print_human(&resp),
    }
    Ok(())
}

fn print_plain(resp: &Value) {
    for c in resp["clusters"].as_array().iter().flat_map(|a| a.iter()) {
        println!(
            "{}\t{:.3}\t{}",
            c["cluster_id"].as_str().unwrap_or(""),
            c["confidence"].as_f64().unwrap_or(0.0),
            c["hoist_target"].as_str().unwrap_or(""),
        );
    }
}

fn print_human(resp: &Value) {
    let clusters = resp["clusters"].as_array().cloned().unwrap_or_default();
    if clusters.is_empty() {
        println!("No cross-repo patterns found.");
        return;
    }
    let total = clusters.len();
    println!("{total} cross-repo pattern cluster(s) detected:\n");
    for c in &clusters {
        let id = c["cluster_id"].as_str().unwrap_or("?");
        let conf = c["confidence"].as_f64().unwrap_or(0.0);
        let target = c["hoist_target"].as_str().unwrap_or("?");
        println!(
            "Cluster {} — confidence {:.1}% — hoist → '{target}'",
            &id[..id.len().min(12)],
            conf * 100.0,
        );
        if let Some(members) = c["members"].as_array() {
            for m in members {
                println!(
                    "  {} :: {} ({})",
                    m["repo"].as_str().unwrap_or("?"),
                    m["name"].as_str().unwrap_or("?"),
                    m["kind"].as_str().unwrap_or("?"),
                );
            }
        }
        println!();
    }
}
