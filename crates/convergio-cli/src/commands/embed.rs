//! `cvg embed *` — semantic retrieval client (ADR-0038, F1-γ).
//!
//! Pure HTTP client: every subcommand round-trips to the daemon. The
//! actual embedding model and storage live there, never in the CLI.

use crate::commands::{Client, OutputMode};
use anyhow::{Context, Result};
use clap::Subcommand;
use serde_json::{json, Value};

/// `cvg embed` subcommands.
#[derive(Debug, Subcommand)]
pub enum EmbedCommand {
    /// Show how many embeddings the daemon is holding (optionally
    /// filtered by `--repo`).
    Stats {
        /// Limit the count to one repo.
        #[arg(long)]
        repo: Option<String>,
    },
    /// Load the configured model and embed a sentinel string. Useful
    /// to trigger the one-time ONNX download out of a request hot
    /// path. Reports model id, dim, and elapsed ms.
    Warm,
    /// Walk a directory, embed eligible files, upsert into the store.
    /// Idempotent: rows with unchanged source hash are skipped.
    Build {
        /// Repo identifier written into `graph_node_embeddings.repo`.
        #[arg(long, default_value = "convergio")]
        repo: String,
        /// Directory to walk (use `.` for the current workspace).
        #[arg(long)]
        root: std::path::PathBuf,
        /// Override the default file-extension allowlist
        /// (`rs,md,sql,toml,ftl,yaml,yml`).
        #[arg(long, value_delimiter = ',')]
        extensions: Option<Vec<String>>,
        /// Override the default per-file truncation (200 lines).
        #[arg(long)]
        max_lines: Option<usize>,
    },
    /// Run a semantic-only nearest-neighbor query.
    ForTask {
        /// Free-text query (typically the task body).
        query: String,
        /// Top-K neighbors to return. Capped server-side at 100.
        #[arg(long, default_value_t = 25)]
        top_k: usize,
    },
}

/// Dispatch a `cvg embed` subcommand to the daemon.
pub async fn run(client: &Client, output: OutputMode, sub: EmbedCommand) -> Result<()> {
    match sub {
        EmbedCommand::Stats { repo } => stats(client, output, repo).await,
        EmbedCommand::Warm => warm(client, output).await,
        EmbedCommand::Build {
            repo,
            root,
            extensions,
            max_lines,
        } => build(client, output, repo, root, extensions, max_lines).await,
        EmbedCommand::ForTask { query, top_k } => for_task(client, output, query, top_k).await,
    }
}

async fn stats(client: &Client, output: OutputMode, repo: Option<String>) -> Result<()> {
    let path = match repo.as_deref() {
        Some(r) => format!("/v1/embed/stats?repo={}", urlencode(r)),
        None => "/v1/embed/stats".to_string(),
    };
    let body: Value = client.get(&path).await.context("GET /v1/embed/stats")?;
    print_value(output, &body)
}

async fn warm(client: &Client, output: OutputMode) -> Result<()> {
    let body: Value = client
        .post("/v1/embed/warm", &json!({}))
        .await
        .context("POST /v1/embed/warm")?;
    print_value(output, &body)
}

async fn build(
    client: &Client,
    output: OutputMode,
    repo: String,
    root: std::path::PathBuf,
    extensions: Option<Vec<String>>,
    max_lines: Option<usize>,
) -> Result<()> {
    let mut payload = json!({
        "repo": repo,
        "root": root.to_string_lossy(),
    });
    if let Some(exts) = extensions {
        payload["extensions"] = json!(exts);
    }
    if let Some(ml) = max_lines {
        payload["max_lines"] = json!(ml);
    }
    let body: Value = client
        .post("/v1/embed/build", &payload)
        .await
        .context("POST /v1/embed/build")?;
    print_value(output, &body)
}

async fn for_task(client: &Client, output: OutputMode, query: String, top_k: usize) -> Result<()> {
    let body: Value = client
        .post(
            "/v1/embed/for-task",
            &json!({"query": query, "top_k": top_k}),
        )
        .await
        .context("POST /v1/embed/for-task")?;
    print_value(output, &body)
}

fn print_value(_output: OutputMode, body: &Value) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(body)?);
    Ok(())
}

fn urlencode(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            c if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~') => c.to_string(),
            _ => format!("%{:02X}", c as u32),
        })
        .collect()
}
