//! `cvg fleet duplicates` — cross-repo near-exact duplicate pairs (ADR-0038, F2-10).

use super::{Client, OutputMode};
use anyhow::Result;
use serde_json::Value;

/// Entry point called from `fleet::run`.
pub async fn run(
    client: &Client,
    output: OutputMode,
    cosine: f64,
    repo_pair: Option<&str>,
    diff_preview: bool,
) -> Result<()> {
    let mut url = format!("/v1/fleet/duplicates?cosine={cosine}");
    if let Some(rp) = repo_pair {
        url.push_str(&format!("&repo_pair={rp}"));
    }
    if diff_preview {
        url.push_str("&diff_preview=true");
    }
    let resp: Value = client.get(&url).await?;
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&resp)?),
        OutputMode::Plain => print_plain(&resp),
        OutputMode::Human => print_human(&resp, diff_preview),
    }
    Ok(())
}

fn print_plain(resp: &Value) {
    for p in resp["pairs"].as_array().iter().flat_map(|a| a.iter()) {
        println!(
            "{}/{}\t{}/{}\t{:.4}",
            p["repo_a"].as_str().unwrap_or(""),
            p["name_a"].as_str().unwrap_or(""),
            p["repo_b"].as_str().unwrap_or(""),
            p["name_b"].as_str().unwrap_or(""),
            p["score"].as_f64().unwrap_or(0.0),
        );
    }
}

fn print_human(resp: &Value, diff_preview: bool) {
    let pairs = resp["pairs"].as_array().cloned().unwrap_or_default();
    if pairs.is_empty() {
        println!("No duplicate pairs found.");
        return;
    }
    let total = pairs.len();
    println!("{total} cross-repo duplicate pair(s):\n");
    for p in &pairs {
        let ra = p["repo_a"].as_str().unwrap_or("?");
        let na = p["name_a"].as_str().unwrap_or("?");
        let ka = p["kind_a"].as_str().unwrap_or("?");
        let rb = p["repo_b"].as_str().unwrap_or("?");
        let nb = p["name_b"].as_str().unwrap_or("?");
        let kb = p["kind_b"].as_str().unwrap_or("?");
        let score = p["score"].as_f64().unwrap_or(0.0);
        println!("{ra}::{na} ({ka})  ↔  {rb}::{nb} ({kb})  [score {score:.4}]");
        if diff_preview {
            if let Some(lines) = p["diff_preview"].as_array() {
                for line in lines {
                    if let Some(s) = line.as_str() {
                        println!("  {s}");
                    }
                }
            }
            println!();
        }
    }
}
