//! `cvg fleet ...` — fleet repo + plan management (ADR-0038, F2/F3).
//! Pure HTTP; the daemon owns the stores.

use super::fleet_detect::{default_parser, detect_language, read_derives_from};
use super::{fleet_plan::FleetPlanCommand, Client, OutputMode};
use anyhow::{Context, Result};
use clap::Subcommand;
use serde_json::{json, Value};

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
    /// Sweep operator-side residue (orphan worktrees + stale `agent/*` branches).
    /// See `fleet_cleanup` module doc for rationale.
    Cleanup {
        /// Preview without touching the filesystem.
        #[arg(long)]
        dry_run: bool,
    },
    /// Detect pattern clusters spanning ≥2 repos over `similar_to` edges.
    Patterns {
        /// Minimum distinct repos a cluster must span (default 2).
        #[arg(long, default_value_t = 2)]
        min_repos: usize,
    },
    /// Cross-repo plan management.
    #[command(subcommand)]
    Plan(FleetPlanCommand),
    /// List cross-repo duplicate pairs.
    /// Dispatch one executor tick scoped to a fleet repo.
    Dispatch {
        /// Registered fleet repo name.
        repo: String,
        /// Do not spawn workers; return tracker-only status.
        #[arg(long)]
        no_dispatch: bool,
        /// Executor mode. `none` is tracker-only.
        #[arg(long, value_enum, default_value_t = super::dispatch::ExecutorMode::Default)]
        executor: super::dispatch::ExecutorMode,
    },
    Duplicates {
        /// Cosine similarity threshold (default 0.95).
        #[arg(long, default_value_t = 0.95)]
        cosine: f64,
        /// Restrict to one repo pair, e.g. "alpha:beta".
        #[arg(long)]
        repo_pair: Option<String>,
        /// Show 1–3 line semantic diff preview per pair.
        #[arg(long)]
        diff_preview: bool,
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
            super::fleet_build::run(client, output, refresh_similarity).await
        }
        FleetCommand::Cleanup { dry_run } => super::fleet_cleanup::run(output, dry_run),
        FleetCommand::Patterns { min_repos } => {
            super::fleet_patterns::run(client, output, min_repos).await
        }
        FleetCommand::Plan(cmd) => super::fleet_plan::run(client, output, cmd).await,
        FleetCommand::Dispatch {
            repo,
            no_dispatch,
            executor,
        } => super::fleet_dispatch::run(client, output, &repo, no_dispatch, executor).await,
        FleetCommand::Duplicates {
            cosine,
            repo_pair,
            diff_preview,
        } => {
            super::fleet_duplicates::run(client, output, cosine, repo_pair.as_deref(), diff_preview)
                .await
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
