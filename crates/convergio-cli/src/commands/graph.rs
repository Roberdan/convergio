//! `cvg graph ...` — Tier-3 retrieval client (ADR-0014).
//!
//! Pure HTTP. The daemon owns the SQLite store and the syn parser;
//! the CLI just renders. Renderers live in [`super::graph_render`]
//! to keep this file under the 300-line cap.

use super::graph_render::{
    render_build_human, render_cluster_human, render_drift_human, render_pack_human, render_plain,
};
use super::{Client, OutputMode};
use anyhow::Result;
use clap::Subcommand;
use serde_json::Value;

/// Graph subcommands.
#[derive(Subcommand)]
pub enum GraphCommand {
    /// Walk the workspace and refresh the graph in the daemon's SQLite.
    Build {
        /// Repo root (defaults to daemon's cwd).
        #[arg(long)]
        manifest_dir: Option<String>,
        /// Re-parse every file even if mtime unchanged.
        #[arg(long)]
        force: bool,
    },
    /// Print the current node + edge counts.
    Stats,
    /// Emit a context-pack scoped to one task.
    ForTask {
        /// Task id.
        task_id: String,
        /// Cap on matched-node count.
        #[arg(long)]
        node_limit: Option<usize>,
        /// Cap on the file-union token estimate.
        #[arg(long)]
        token_budget: Option<u64>,
        /// Primary crate scope, e.g. `convergio-graph`.
        #[arg(long = "crate")]
        crate_name: Option<String>,
        /// Comma-separated related crates.
        #[arg(long)]
        related_crates: Option<String>,
        /// Comma-separated required ADR ids or paths.
        #[arg(long)]
        adr_required: Option<String>,
        /// Comma-separated required documentation paths.
        #[arg(long)]
        docs_required: Option<String>,
        /// Named validation profile for the specialized agent.
        #[arg(long)]
        validation_profile: Option<String>,
        /// Fuse substring + semantic retrieval via RRF (ADR-0038
        /// § 5.5). Adds a `hybrid` block to the response with hits
        /// ranked across both retrievers and per-source provenance.
        #[arg(long)]
        semantic: bool,
        /// Top-K semantic neighbors fed into the RRF fusion.
        /// Capped server-side at 100. Ignored without `--semantic`.
        #[arg(long, default_value_t = 25)]
        semantic_top_k: usize,
        /// Linear-blend weight for structural signal (`0.0`–`1.0`).
        /// When set, overrides RRF: score = alpha × structural + (1−alpha) × semantic.
        /// Implies `--semantic`. Mitigates regressions when substring retrieval saturates.
        #[arg(long)]
        alpha: Option<f64>,
    },
    /// Compare ADR claims (touches_crates) against the actual git
    /// diff. Reports drift (touched but not declared) and ghosts
    /// (declared but not touched).
    Drift {
        /// Git ref to diff against (default `origin/main`).
        #[arg(long)]
        since: Option<String>,
        /// Optional ADR id to scope the declared set.
        #[arg(long)]
        adr: Option<String>,
        /// Repo root (defaults to daemon's cwd).
        #[arg(long)]
        repo_root: Option<String>,
    },
    /// Run community detection over the named crate's file graph.
    /// Suggests split seams when the crate is approaching the
    /// legibility cap.
    Cluster {
        /// Crate to inspect (e.g. `convergio-durability`).
        crate_name: String,
        /// Optional LOC budget; communities above the budget are
        /// flagged.
        #[arg(long)]
        target_loc: Option<u64>,
    },
}

/// Entry point.
pub async fn run(client: &Client, output: OutputMode, cmd: GraphCommand) -> Result<()> {
    match cmd {
        GraphCommand::Build {
            manifest_dir,
            force,
        } => build(client, output, manifest_dir, force).await,
        GraphCommand::Stats => stats(client, output).await,
        GraphCommand::ForTask {
            task_id,
            node_limit,
            token_budget,
            crate_name,
            related_crates,
            adr_required,
            docs_required,
            validation_profile,
            semantic,
            semantic_top_k,
            alpha,
        } => {
            let hints = ForTaskHints {
                crate_name,
                related_crates,
                adr_required,
                docs_required,
                validation_profile,
            };
            for_task(
                client,
                output,
                &task_id,
                node_limit,
                token_budget,
                hints,
                semantic,
                semantic_top_k,
                alpha,
            )
            .await
        }
        GraphCommand::Drift {
            since,
            adr,
            repo_root,
        } => drift(client, output, since, adr, repo_root).await,
        GraphCommand::Cluster {
            crate_name,
            target_loc,
        } => cluster(client, output, &crate_name, target_loc).await,
    }
}

async fn drift(
    client: &Client,
    output: OutputMode,
    since: Option<String>,
    adr: Option<String>,
    repo_root: Option<String>,
) -> Result<()> {
    let mut path = String::from("/v1/graph/drift?");
    if let Some(s) = since {
        path.push_str(&format!("since={s}&"));
    }
    if let Some(a) = adr {
        path.push_str(&format!("adr={a}&"));
    }
    if let Some(r) = repo_root {
        path.push_str(&format!("repo_root={r}&"));
    }
    let report: Value = client.get(&path).await?;
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&report)?),
        OutputMode::Plain => render_plain(&report),
        OutputMode::Human => render_drift_human(&report),
    }
    Ok(())
}

async fn build(
    client: &Client,
    output: OutputMode,
    manifest_dir: Option<String>,
    force: bool,
) -> Result<()> {
    let body = serde_json::json!({
        "manifest_dir": manifest_dir,
        "force": force,
    });
    let report: Value = client.post("/v1/graph/build", &body).await?;
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&report)?),
        OutputMode::Plain => render_plain(&report),
        OutputMode::Human => render_build_human(&report),
    }
    Ok(())
}

async fn stats(client: &Client, output: OutputMode) -> Result<()> {
    let body: Value = client.get("/v1/graph/stats").await?;
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&body)?),
        OutputMode::Plain => render_plain(&body),
        OutputMode::Human => println!(
            "Graph: {} nodes, {} edges.",
            body.get("nodes").and_then(Value::as_u64).unwrap_or(0),
            body.get("edges").and_then(Value::as_u64).unwrap_or(0),
        ),
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn for_task(
    client: &Client,
    output: OutputMode,
    task_id: &str,
    node_limit: Option<usize>,
    token_budget: Option<u64>,
    hints: ForTaskHints,
    semantic: bool,
    semantic_top_k: usize,
    alpha: Option<f64>,
) -> Result<()> {
    let mut path = format!("/v1/graph/for-task/{task_id}?");
    if let Some(n) = node_limit {
        path.push_str(&format!("node_limit={n}&"));
    }
    if let Some(t) = token_budget {
        path.push_str(&format!("token_budget={t}&"));
    }
    // --alpha implies --semantic
    if semantic || alpha.is_some() {
        path.push_str("semantic=1&");
        path.push_str(&format!("semantic_top_k={semantic_top_k}&"));
    }
    if let Some(a) = alpha {
        path.push_str(&format!("alpha={a}&"));
    }
    append_query_param(&mut path, "crate", hints.crate_name.as_deref());
    append_query_param(&mut path, "related_crates", hints.related_crates.as_deref());
    append_query_param(&mut path, "adr_required", hints.adr_required.as_deref());
    append_query_param(&mut path, "docs_required", hints.docs_required.as_deref());
    append_query_param(
        &mut path,
        "validation_profile",
        hints.validation_profile.as_deref(),
    );
    let pack: Value = client.get(&path).await?;
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&pack)?),
        OutputMode::Plain => render_plain(&pack),
        OutputMode::Human => render_pack_human(&pack),
    }
    Ok(())
}

struct ForTaskHints {
    crate_name: Option<String>,
    related_crates: Option<String>,
    adr_required: Option<String>,
    docs_required: Option<String>,
    validation_profile: Option<String>,
}

fn append_query_param(path: &mut String, name: &str, value: Option<&str>) {
    if let Some(value) = value {
        path.push_str(name);
        path.push('=');
        path.push_str(&encode_query_value(value));
        path.push('&');
    }
}

fn encode_query_value(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(byte as char);
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

async fn cluster(
    client: &Client,
    output: OutputMode,
    crate_name: &str,
    target_loc: Option<u64>,
) -> Result<()> {
    let mut path = format!("/v1/graph/cluster/{crate_name}?");
    if let Some(t) = target_loc {
        path.push_str(&format!("target_loc={t}"));
    }
    let report: Value = client.get(&path).await?;
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&report)?),
        OutputMode::Plain => render_plain(&report),
        OutputMode::Human => render_cluster_human(&report),
    }
    Ok(())
}
