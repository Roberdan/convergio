//! `cvg coherence fleet` — cross-repo schema sub-verifier (P1-7,
//! issue #177).
//!
//! With F2 fleet pipeline in main, no automatic check that
//! `~/.convergio/v3/fleet.toml` stays consistent with the workspace.
//! This verifier walks `fleet.toml` and reports findings:
//!
//! - `missing_path` — `repo.path` does not exist on disk.
//! - `missing_retrieval_golden` — no
//!   `tests/fixtures/retrieval-golden/<repo.name>/` directory
//!   (gates the F2-12 fixtures wired into CI).
//! - `dangling_derives_from` — `repo.derives_from` references a
//!   name not in the fleet.
//! - `multiple_engine_roots` — more than one repo with
//!   `RepoRole::Engine` (the F2 design allows exactly one).
//!
//! Default mode is advisory (exit 0). `--strict` flips the exit code
//! to 1 when any of the four findings appears.

use crate::OutputMode;
use anyhow::{Context, Result};
use convergio_fleet::config::{FleetConfig, RepoEntry, RepoRole};
use convergio_i18n::Bundle;
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// One finding row.
#[derive(Debug, Clone, Serialize)]
pub struct Row {
    /// Repo name (or empty for fleet-wide findings like multi-engine).
    pub repo: String,
    /// Finding key — see module doc.
    pub kind: &'static str,
    /// Short evidence string.
    pub evidence: String,
}

/// Verifier report.
#[derive(Debug, Default, Serialize)]
pub struct Report {
    /// Path to the fleet.toml that was inspected.
    pub fleet_toml: String,
    /// Number of repos declared in the file.
    pub repos: usize,
    /// Findings, one row per violation.
    pub rows: Vec<Row>,
}

/// Run the verifier.
pub async fn run(
    bundle: &Bundle,
    output: OutputMode,
    config_path: Option<PathBuf>,
    strict: bool,
) -> Result<()> {
    let path = config_path.unwrap_or_else(default_fleet_toml);
    let report = build_report(&path)?;
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&report)?),
        OutputMode::Plain => render_plain(&report),
        OutputMode::Human => render_human(&report, bundle),
    }
    if strict && !report.rows.is_empty() {
        std::process::exit(1);
    }
    Ok(())
}

fn default_fleet_toml() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".convergio/v3/fleet.toml")
}

pub(crate) fn build_report(path: &Path) -> Result<Report> {
    let mut report = Report {
        fleet_toml: path.display().to_string(),
        ..Report::default()
    };
    if !path.exists() {
        report.rows.push(Row {
            repo: String::new(),
            kind: "missing_fleet_toml",
            evidence: format!("no file at {}", path.display()),
        });
        return Ok(report);
    }
    let body = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let cfg: FleetConfig =
        toml::from_str(&body).with_context(|| format!("parse {}", path.display()))?;
    report.repos = cfg.repos.len();
    let names: BTreeSet<String> = cfg.repos.iter().map(|r| r.name.clone()).collect();
    let mut engine_count = 0usize;
    for repo in &cfg.repos {
        report.rows.extend(check_repo(repo, &names));
        if repo.role == RepoRole::Engine {
            engine_count += 1;
        }
    }
    if engine_count > 1 {
        report.rows.push(Row {
            repo: String::new(),
            kind: "multiple_engine_roots",
            evidence: format!("{engine_count} repos with role=engine; expected at most 1"),
        });
    }
    Ok(report)
}

fn check_repo(repo: &RepoEntry, names: &BTreeSet<String>) -> Vec<Row> {
    let mut rows = Vec::new();
    if !PathBuf::from(&repo.path).exists() {
        rows.push(Row {
            repo: repo.name.clone(),
            kind: "missing_path",
            evidence: format!("path {} does not exist", repo.path),
        });
    }
    let golden = PathBuf::from(&repo.path)
        .join("tests/fixtures/retrieval-golden")
        .join(&repo.name);
    if !golden.exists() {
        rows.push(Row {
            repo: repo.name.clone(),
            kind: "missing_retrieval_golden",
            evidence: format!("no fixtures at {}", golden.display()),
        });
    }
    if let Some(parent) = &repo.derives_from {
        if !names.contains(parent) {
            rows.push(Row {
                repo: repo.name.clone(),
                kind: "dangling_derives_from",
                evidence: format!("derives_from '{parent}' is not in the fleet"),
            });
        }
    }
    rows
}

fn render_human(report: &Report, _bundle: &Bundle) {
    println!(
        "cvg coherence fleet — {} repo(s) in {}",
        report.repos, report.fleet_toml
    );
    if report.rows.is_empty() {
        println!("  no findings — clean.");
        return;
    }
    println!("  {} finding(s):", report.rows.len());
    for r in &report.rows {
        let label = if r.repo.is_empty() {
            "<fleet>".to_string()
        } else {
            r.repo.clone()
        };
        println!("    {:<20} {:<30} {}", label, r.kind, r.evidence);
    }
}

fn render_plain(report: &Report) {
    for r in &report.rows {
        println!("{}\t{}\t{}", r.repo, r.kind, r.evidence);
    }
    println!("# repos={} findings={}", report.repos, report.rows.len());
}
