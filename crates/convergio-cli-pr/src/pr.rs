//! `cvg pr ...` — local PR queue dashboard with conflict detection.
//!
//! `cvg pr stack` reads open GitHub PRs via `gh`, parses each PR
//! body for the `## Files touched` machine-readable manifest (see
//! `.github/pull_request_template.md`), computes the file-overlap
//! matrix, and suggests a merge order that minimises rebase pain.
//!
//! Read-only by design. Never merges, never closes, never pushes.
//! CONSTITUTION § Merge discipline: agents may not merge without
//! explicit user confirmation.
//!
//! Renderers live in the sibling [`super::pr_render`] module to keep
//! both files under the 300-line cap.

use super::pr_diff::{compare_manifest, fetch_pr_files};
use super::pr_merge::MergeArgs;
use super::pr_parse::parse_manifest;
use super::pr_render;
use super::{Client, OutputMode};
use anyhow::{Context, Result};
use clap::Subcommand;
use convergio_i18n::Bundle;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;

/// Pr subcommands.
#[derive(Subcommand)]
pub enum PrCommand {
    /// Show open PRs, the file-conflict matrix, and a suggested
    /// merge order. Read-only.
    Stack,
    /// Sync plan tasks against merged PRs that declare `Tracks:
    /// <task-uuid>` lines. Transitions pending tasks to submitted
    /// when their tracking PR has merged. Closes friction-log F35
    /// (plan-vs-merged-PR drift). See ADR-0023 + PRD-001 Artefact 4
    /// for the structural pattern this implements.
    Sync {
        /// Plan id whose tasks to sync.
        #[arg(value_name = "PLAN_ID")]
        plan: String,
        /// Agent id to record on the transition. Falls back to
        /// `CONVERGIO_AGENT_ID` env var or anonymous.
        #[arg(long, env = "CONVERGIO_AGENT_ID")]
        agent_id: Option<String>,
    },
    /// Merge a PR with 8-check pre-flight, branch + worktree
    /// cleanup, optional sub-agent retire, and a transactional
    /// `merge_record` evidence row on every task tracked by the PR
    /// body. On AUTO-block conflict the wrapper aborts with an
    /// actionable hint (in-process auto-resolve is a follow-up).
    /// Subsumes A2/B1/B5/C5/C6/F3 from the 2026-05-04 retrospective.
    /// See `~/Desktop/convergio-retrospective-2026-05-04.md` §2 P0-1.
    Merge(MergeArgs),
}

/// Run a pr subcommand.
pub async fn run(
    client: &Client,
    bundle: &Bundle,
    output: OutputMode,
    cmd: PrCommand,
) -> Result<()> {
    match cmd {
        PrCommand::Stack => stack(bundle, output).await,
        PrCommand::Sync { plan, agent_id } => {
            super::pr_sync::run(client, plan, agent_id, output).await
        }
        PrCommand::Merge(args) => super::pr_merge::run(client, output, args).await,
    }
}

async fn stack(bundle: &Bundle, output: OutputMode) -> Result<()> {
    let prs = fetch_prs().context("`gh pr list` — is gh installed and authenticated?")?;
    let analysed: Vec<AnalysedPr> = prs
        .iter()
        .map(|v| analyse_pr_with_diff(v).unwrap_or_else(|_| analyse_pr(v)))
        .collect();
    let order = suggest_merge_order(&analysed);
    pr_render::render(bundle, output, &analysed, &order)
}

/// Status of a PR's `## Files touched` manifest vs the real diff.
/// `pub(crate)` so the sibling `pr_render` module can read it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManifestStatus {
    /// Manifest covers exactly the diffed files.
    Match,
    /// Manifest is missing or empty.
    Missing,
    /// Manifest disagrees with the diff (extra or missing entries).
    Mismatch,
}

/// One PR after parsing its body for the Files-touched manifest.
/// `pub(crate)` so the sibling `pr_render` module can read it.
pub(crate) struct AnalysedPr {
    pub number: i64,
    pub title: String,
    pub files: BTreeSet<String>,
    pub depends_on: BTreeSet<i64>,
    pub manifest_status: ManifestStatus,
}

fn fetch_prs() -> Result<Vec<Value>> {
    let out = Command::new("gh")
        .args([
            "pr",
            "list",
            "--state",
            "open",
            "--json",
            "number,title,body",
        ])
        .output()
        .context("spawn gh")?;
    if !out.status.success() {
        anyhow::bail!(
            "gh pr list failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let arr: Vec<Value> = serde_json::from_slice(&out.stdout).context("parse gh output")?;
    Ok(arr)
}

/// Best-effort: pull the real diff for one PR and cross-check.
/// Falls back to manifest-only via [`analyse_pr`] on any gh failure.
fn analyse_pr_with_diff(value: &Value) -> Result<AnalysedPr> {
    let number = value.get("number").and_then(Value::as_i64).unwrap_or(0);
    let title = value
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let body = value.get("body").and_then(Value::as_str).unwrap_or("");
    let manifest = parse_manifest(body);
    let diff_files = fetch_pr_files(number)?;
    let manifest_status = compare_manifest(&manifest, &diff_files);
    Ok(AnalysedPr {
        number,
        title,
        // Trust the diff when it disagrees with the manifest — the
        // diff is ground truth, the manifest is human-authored.
        files: diff_files,
        depends_on: manifest.depends_on,
        manifest_status,
    })
}

fn analyse_pr(value: &Value) -> AnalysedPr {
    let number = value.get("number").and_then(Value::as_i64).unwrap_or(0);
    let title = value
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let body = value.get("body").and_then(Value::as_str).unwrap_or("");
    let manifest = parse_manifest(body);
    let manifest_status = if manifest.files.is_empty() {
        ManifestStatus::Missing
    } else {
        ManifestStatus::Match
    };
    AnalysedPr {
        number,
        title,
        files: manifest.files,
        depends_on: manifest.depends_on,
        manifest_status,
    }
}

/// Compute the file overlap between every pair, then a topological
/// merge order: bottom-up by `Depends on` edges, with overlap-pairs
/// alphabetised stable so the output is deterministic.
fn suggest_merge_order(prs: &[AnalysedPr]) -> Vec<i64> {
    let mut by_id: BTreeMap<i64, &AnalysedPr> = BTreeMap::new();
    for p in prs {
        by_id.insert(p.number, p);
    }
    let mut visited: BTreeSet<i64> = BTreeSet::new();
    let mut order: Vec<i64> = Vec::new();
    fn visit(
        id: i64,
        by_id: &BTreeMap<i64, &AnalysedPr>,
        visited: &mut BTreeSet<i64>,
        order: &mut Vec<i64>,
    ) {
        if !visited.insert(id) {
            return;
        }
        if let Some(pr) = by_id.get(&id) {
            for &dep in &pr.depends_on {
                visit(dep, by_id, visited, order);
            }
        }
        order.push(id);
    }
    let mut keys: Vec<i64> = by_id.keys().copied().collect();
    keys.sort_by_key(|id| {
        by_id
            .get(id)
            .map(|p| (count_overlap(p, prs), p.number))
            .unwrap_or((0, 0))
    });
    for k in keys {
        visit(k, &by_id, &mut visited, &mut order);
    }
    order
}

fn count_overlap(target: &AnalysedPr, all: &[AnalysedPr]) -> usize {
    all.iter()
        .filter(|p| p.number != target.number)
        .map(|p| target.files.intersection(&p.files).count())
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pr(number: i64, depends_on: &[i64]) -> AnalysedPr {
        AnalysedPr {
            number,
            title: format!("pr-{number}"),
            files: BTreeSet::new(),
            depends_on: depends_on.iter().copied().collect(),
            manifest_status: ManifestStatus::Missing,
        }
    }

    #[test]
    fn merge_order_respects_explicit_dependencies() {
        let order = suggest_merge_order(&[pr(2, &[1]), pr(1, &[])]);
        let pos1 = order.iter().position(|&n| n == 1).unwrap();
        let pos2 = order.iter().position(|&n| n == 2).unwrap();
        assert!(pos1 < pos2, "PR 1 must merge before PR 2 (its dependent)");
    }
}
