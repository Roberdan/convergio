//! `cvg coherence adrs` — ADR status vs implementation cross-check.
//!
//! Heuristic sub-verifier that flags ADRs whose declared `status:`
//! frontmatter does not match the implementation reality:
//!
//! - `accepted_no_evidence` — `status: accepted` with non-empty
//!   `touches_crates`, but no obvious mention of `ADR-NNNN` and no
//!   topic-slug match in any of those crates' `src/`.
//! - `proposed_likely_shipped` — `status: proposed`, but a strong
//!   keyword from the ADR slug (or an `ADR-NNNN` comment) appears in
//!   workspace source — advisory only, never strict.
//! - `broken_supersession` — `status: superseded by NNNN` where
//!   ADR-NNNN does not exist on disk.
//!
//! Exit code is advisory by default. With `--strict`, the verifier
//! exits non-zero on `accepted_no_evidence` and `broken_supersession`
//! (these are unambiguous bugs); never on `proposed_likely_shipped`.
//!
//! Internationalised through `convergio-i18n` (P5). Keys live in
//! `crates/convergio-i18n/locales/{en,it}/main.ftl` under
//! `# ---------- CLI: coherence adrs ----------`.

use crate::adrs_scan::{all_crates, find_evidence, parse_superseded_by};
use crate::parse::{load_adrs_full, AdrFull};
use crate::OutputMode;
use anyhow::Result;
use convergio_i18n::Bundle;
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::Path;

/// Emitted finding bucket.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Finding {
    /// `status: accepted` but no on-disk evidence in `touches_crates`.
    AcceptedNoEvidence,
    /// `status: proposed` yet topic / `ADR-NNNN` shows up in source.
    ProposedLikelyShipped,
    /// `status: superseded by NNNN` and ADR NNNN does not exist.
    BrokenSupersession,
}

impl Finding {
    /// Fluent message key for this finding bucket.
    pub fn ftl_key(self) -> &'static str {
        match self {
            Finding::AcceptedNoEvidence => "coherence-adrs-finding-accepted-no-evidence",
            Finding::ProposedLikelyShipped => "coherence-adrs-finding-proposed-likely-shipped",
            Finding::BrokenSupersession => "coherence-adrs-finding-broken-supersession",
        }
    }

    /// Plain-text key (output mode `plain`, JSON, tests).
    pub fn key(self) -> &'static str {
        match self {
            Finding::AcceptedNoEvidence => "accepted_no_evidence",
            Finding::ProposedLikelyShipped => "proposed_likely_shipped",
            Finding::BrokenSupersession => "broken_supersession",
        }
    }

    /// True for findings that flip the exit code under `--strict`.
    pub fn is_strict(self) -> bool {
        matches!(
            self,
            Finding::AcceptedNoEvidence | Finding::BrokenSupersession
        )
    }
}

/// One row in the report.
#[derive(Debug, Clone, Serialize)]
pub struct Row {
    /// ADR id, zero-padded (`"0006"`).
    pub id: String,
    /// Declared status string (verbatim from frontmatter).
    pub declared: String,
    /// Finding bucket.
    pub finding: Finding,
    /// Short human evidence string (file path or hint).
    pub evidence: String,
}

/// Full report.
#[derive(Debug, Serialize)]
pub struct Report {
    /// How many ADRs were inspected.
    pub adrs_checked: usize,
    /// Rows emitted (one per finding).
    pub rows: Vec<Row>,
}

/// Run the verifier against `root` and return a structured report.
pub fn run_check(root: &Path) -> Result<Report> {
    let adrs = load_adrs_full(&root.join("docs/adr"))?;
    let known_ids: BTreeSet<String> = adrs.iter().map(|a| a.id.clone()).collect();
    let mut rows: Vec<Row> = Vec::new();
    for adr in &adrs {
        rows.extend(check_adr(adr, &known_ids, root)?);
    }
    Ok(Report {
        adrs_checked: adrs.len(),
        rows,
    })
}

fn check_adr(adr: &AdrFull, known_ids: &BTreeSet<String>, root: &Path) -> Result<Vec<Row>> {
    let status_lc = adr.status.trim().to_ascii_lowercase();
    if let Some(target) = parse_superseded_by(&status_lc) {
        if !known_ids.contains(&target) {
            return Ok(vec![Row {
                id: adr.id.clone(),
                declared: adr.status.clone(),
                finding: Finding::BrokenSupersession,
                evidence: format!("ADR-{target} not on disk"),
            }]);
        }
        return Ok(Vec::new());
    }
    if status_lc == "accepted" && !adr.touches_crates.is_empty() {
        if find_evidence(&adr.id, &adr.slug, &adr.touches_crates, root)?.is_some() {
            return Ok(Vec::new());
        }
        return Ok(vec![Row {
            id: adr.id.clone(),
            declared: adr.status.clone(),
            finding: Finding::AcceptedNoEvidence,
            evidence: format!("scanned {}: no match", adr.touches_crates.join(",")),
        }]);
    }
    if status_lc == "proposed" {
        let scope = if adr.touches_crates.is_empty() {
            all_crates(root)?
        } else {
            adr.touches_crates.clone()
        };
        if let Some(hit) = find_evidence(&adr.id, &adr.slug, &scope, root)? {
            return Ok(vec![Row {
                id: adr.id.clone(),
                declared: adr.status.clone(),
                finding: Finding::ProposedLikelyShipped,
                evidence: hit,
            }]);
        }
    }
    Ok(Vec::new())
}

/// CLI entry point. Renders + sets exit code under `--strict`.
pub async fn run(bundle: &Bundle, output: OutputMode, root: &Path, strict: bool) -> Result<()> {
    let report = run_check(root)?;
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&report)?),
        OutputMode::Plain => render_plain(&report),
        OutputMode::Human => render_human(&report, bundle),
    }
    if strict && report.rows.iter().any(|r| r.finding.is_strict()) {
        std::process::exit(1);
    }
    Ok(())
}

fn render_human(report: &Report, bundle: &Bundle) {
    println!(
        "{}",
        bundle.t(
            "coherence-adrs-summary",
            &[
                ("checked", &report.adrs_checked.to_string()),
                ("findings", &report.rows.len().to_string()),
            ]
        )
    );
    if report.rows.is_empty() {
        println!("{}", bundle.t("coherence-adrs-empty", &[]));
        return;
    }
    println!("{}", bundle.t("coherence-adrs-table-header", &[]));
    for r in &report.rows {
        let finding = bundle.t(r.finding.ftl_key(), &[]);
        println!(
            "  {:<6} {:<32} {:<28} {}",
            r.id, r.declared, finding, r.evidence
        );
    }
}

fn render_plain(report: &Report) {
    println!(
        "checked={} findings={}",
        report.adrs_checked,
        report.rows.len()
    );
    for r in &report.rows {
        println!(
            "{}\t{}\t{}\t{}",
            r.id,
            r.declared,
            r.finding.key(),
            r.evidence
        );
    }
}
