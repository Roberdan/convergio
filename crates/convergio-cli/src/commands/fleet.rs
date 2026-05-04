//! `cvg fleet ...` — fleet repo management (ADR-0038, F2-6/F2-7).
//!
//! Pure HTTP. The daemon owns the fleet store; the CLI just renders.
//!
//! Subcommands:
//! - `add <path>`   — register a repo (reads `convergio.yaml` for `derives_from`)
//! - `ls`           — list all repos
//! - `enable  <name>`  — re-enable a disabled repo
//! - `disable <name>`  — disable a repo (without removing it)
//! - `build`        — parse + embed all enabled repos (idempotent)

use super::{Client, OutputMode};
use anyhow::{Context, Result};
use clap::Subcommand;
use serde_json::{json, Value};
use std::path::Path;

/// Fleet subcommands.
#[derive(Subcommand)]
pub enum FleetCommand {
    /// Register a repository with the fleet.
    ///
    /// Reads `convergio.yaml` in the target repo (if present) to
    /// pick up the `derives_from` declaration.
    Add {
        /// Absolute path to the repo root.
        path: String,
        /// Short slug (defaults to the directory name).
        #[arg(long)]
        name: Option<String>,
        /// Primary language (e.g. "rust", "typescript", "python").
        #[arg(long)]
        language: Option<String>,
        /// Parser backend ("syn" for Rust, "tree-sitter" for others).
        #[arg(long)]
        parser: Option<String>,
        /// Role in the fleet (engine | library | downstream | sandbox).
        #[arg(long)]
        role: Option<String>,
        /// Parent repo this one derives from (overrides convergio.yaml).
        #[arg(long)]
        derives_from: Option<String>,
    },
    /// List all repos in the fleet.
    Ls,
    /// Enable a previously disabled repo.
    Enable {
        /// Short slug of the repo.
        name: String,
    },
    /// Disable a repo (without removing it).
    Disable {
        /// Short slug of the repo.
        name: String,
    },
    /// Parse and embed all enabled fleet repos.
    ///
    /// Idempotent: already-embedded files are skipped via source-hash
    /// comparison. Use `--refresh-similarity` to also recompute
    /// cross-repo cosine similarity edges.
    Build {
        /// Recompute cross-repo `similar_to` / `duplicates` edges after ingestion.
        #[arg(long)]
        refresh_similarity: bool,
    },
}

/// Entry point.
pub async fn run(client: &Client, output: OutputMode, cmd: FleetCommand) -> Result<()> {
    match cmd {
        FleetCommand::Add {
            path,
            name,
            language,
            parser,
            role,
            derives_from,
        } => {
            add(
                client,
                output,
                path,
                name,
                language,
                parser,
                role,
                derives_from,
            )
            .await
        }
        FleetCommand::Ls => ls(client, output).await,
        FleetCommand::Enable { name } => toggle(client, output, &name, true).await,
        FleetCommand::Disable { name } => toggle(client, output, &name, false).await,
        FleetCommand::Build { refresh_similarity } => {
            fleet_build(client, output, refresh_similarity).await
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn add(
    client: &Client,
    output: OutputMode,
    path: String,
    name: Option<String>,
    language: Option<String>,
    parser: Option<String>,
    role: Option<String>,
    derives_from: Option<String>,
) -> Result<()> {
    let abs_path =
        std::fs::canonicalize(&path).with_context(|| format!("cannot resolve path: {path}"))?;
    let abs_str = abs_path
        .to_str()
        .context("path contains invalid UTF-8")?
        .to_owned();

    let slug = name.unwrap_or_else(|| {
        abs_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("repo")
            .to_owned()
    });

    let lang = language.unwrap_or_else(|| detect_language(&abs_path));
    let par = parser.unwrap_or_else(|| default_parser(&lang));
    let df = derives_from.or_else(|| read_derives_from(&abs_path));

    let body = json!({
        "name":         slug,
        "path":         abs_str,
        "language":     lang,
        "parser":       par,
        "role":         role,
        "derives_from": df,
    });

    let repo: Value = client.post("/v1/fleet/repos", &body).await?;
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&repo)?),
        OutputMode::Plain => println!("{}", repo["name"].as_str().unwrap_or("")),
        OutputMode::Human => print_repo_human(&repo),
    }
    Ok(())
}

async fn ls(client: &Client, output: OutputMode) -> Result<()> {
    let resp: Value = client.get("/v1/fleet/repos").await?;
    let repos = resp["repos"].as_array().cloned().unwrap_or_default();
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&resp)?),
        OutputMode::Plain => {
            for r in &repos {
                println!(
                    "{}\t{}\t{}\t{}",
                    r["name"].as_str().unwrap_or(""),
                    r["path"].as_str().unwrap_or(""),
                    r["language"].as_str().unwrap_or(""),
                    r["role"].as_str().unwrap_or(""),
                );
            }
        }
        OutputMode::Human => print_repo_table(&repos),
    }
    Ok(())
}

async fn toggle(client: &Client, output: OutputMode, name: &str, enabled: bool) -> Result<()> {
    let body = json!({ "enabled": enabled });
    let repo: Value = client
        .patch(&format!("/v1/fleet/repos/{name}"), &body)
        .await?;
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&repo)?),
        OutputMode::Plain => println!(
            "{} {}",
            repo["name"].as_str().unwrap_or(""),
            if enabled { "enabled" } else { "disabled" }
        ),
        OutputMode::Human => {
            let status = if enabled { "enabled" } else { "disabled" };
            println!(
                "Fleet repo '{}' {}.",
                repo["name"].as_str().unwrap_or(name),
                status
            );
        }
    }
    Ok(())
}

fn print_repo_human(r: &Value) {
    println!(
        "Added: {} ({}) — {} [{}]",
        r["name"].as_str().unwrap_or("?"),
        r["path"].as_str().unwrap_or("?"),
        r["language"].as_str().unwrap_or("?"),
        r["role"].as_str().unwrap_or("?"),
    );
    if let Some(df) = r["derives_from"].as_str() {
        println!("  derives_from: {df}");
    }
}

fn print_repo_table(repos: &[Value]) {
    if repos.is_empty() {
        println!("No repos in fleet.");
        return;
    }
    println!(
        "{:<20} {:<12} {:<12} {:<10} {:<24} PATH",
        "NAME", "LANGUAGE", "ROLE", "ENABLED", "LAST BUILD"
    );
    println!("{}", "-".repeat(90));
    for r in repos {
        let enabled = r["enabled"].as_bool().unwrap_or(true);
        let last = r["last_built_at"].as_str().unwrap_or("—");
        println!(
            "{:<20} {:<12} {:<12} {:<10} {:<24} {}",
            r["name"].as_str().unwrap_or("?"),
            r["language"].as_str().unwrap_or("?"),
            r["role"].as_str().unwrap_or("?"),
            if enabled { "yes" } else { "no" },
            last,
            r["path"].as_str().unwrap_or("?"),
        );
    }
}

/// Read `derives_from` from `<repo>/convergio.yaml` without pulling in a YAML
/// dep. The file is expected to have a simple `key: value` format.
fn read_derives_from(repo_root: &Path) -> Option<String> {
    let yaml_path = repo_root.join("convergio.yaml");
    let content = std::fs::read_to_string(&yaml_path).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("derives_from:") {
            let value = rest.trim().trim_matches('"').trim_matches('\'').to_owned();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}

fn detect_language(repo_root: &Path) -> String {
    if repo_root.join("Cargo.toml").exists() {
        return "rust".to_owned();
    }
    if repo_root.join("package.json").exists() {
        return "typescript".to_owned();
    }
    if repo_root.join("pyproject.toml").exists() || repo_root.join("setup.py").exists() {
        return "python".to_owned();
    }
    "unknown".to_owned()
}

fn default_parser(language: &str) -> String {
    if language == "rust" {
        "syn".to_owned()
    } else {
        "tree-sitter".to_owned()
    }
}

async fn fleet_build(client: &Client, output: OutputMode, refresh_similarity: bool) -> Result<()> {
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
            let edges = resp["similar_edges_written"].as_u64().unwrap_or(0);
            let model = resp["model"].as_str().unwrap_or("?");
            println!("Fleet build complete ({model}). Repos: {processed}");
            println!("  Files embedded: {embedded}  skipped: {skipped}");
            if refresh_similarity {
                println!("  Similarity edges: {edges}");
            }
        }
    }
    Ok(())
}
