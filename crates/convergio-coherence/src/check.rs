//! Implementation of `cvg coherence check` (ADR/workspace frontmatter check).
//!
//! Extracted from `coherence.rs` to keep that file under the 300-line cap.
//! Called via [`crate::coherence`] dispatch only.

use crate::body::{scan_body, walk_markdown, BodyViolation};
use crate::parse::{load_adrs, parse_index, parse_workspace_members};
use anyhow::Result;
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Debug, Serialize)]
pub(crate) struct Report {
    pub adrs_checked: usize,
    pub crates_known: usize,
    pub index_entries: usize,
    pub docs_scanned: usize,
    pub violations: Vec<Violation>,
}

#[derive(Debug, Serialize)]
pub(crate) struct Violation {
    pub file: String,
    pub kind: String,
    pub detail: String,
}

pub(crate) fn run_check(root: &Path) -> Result<Report> {
    let adrs = load_adrs(&root.join("docs/adr"))?;
    let index = parse_index(&root.join("docs/adr/README.md"))?;
    let crates = parse_workspace_members(&root.join("Cargo.toml"))?;
    let known_ids: BTreeSet<String> = adrs.iter().map(|a| a.id.clone()).collect();

    let mut violations: Vec<Violation> = Vec::new();
    for adr in &adrs {
        for ref_id in &adr.related_adrs {
            if !known_ids.contains(ref_id) {
                violations.push(Violation {
                    file: adr.path.clone(),
                    kind: "missing_adr_ref".into(),
                    detail: format!("references ADR {ref_id} which does not exist"),
                });
            }
        }
        for crate_name in &adr.touches_crates {
            if !crates.contains(crate_name) {
                violations.push(Violation {
                    file: adr.path.clone(),
                    kind: "missing_crate_ref".into(),
                    detail: format!(
                        "touches_crates references '{crate_name}' which is not in workspace.members"
                    ),
                });
            }
        }
        match index.get(&adr.id) {
            Some(idx_status) => {
                if !statuses_match(&adr.status, idx_status) {
                    violations.push(Violation {
                        file: adr.path.clone(),
                        kind: "status_mismatch".into(),
                        detail: format!(
                            "ADR file status '{}' != index status '{}'",
                            adr.status, idx_status
                        ),
                    });
                }
            }
            None => violations.push(Violation {
                file: adr.path.clone(),
                kind: "missing_from_index".into(),
                detail: format!(
                    "ADR {} exists on disk but has no row in docs/adr/README.md",
                    adr.id
                ),
            }),
        }
    }

    let mut docs_scanned = 0usize;
    for (rel, body) in walk_markdown(root)? {
        docs_scanned += 1;
        for v in scan_body(&rel, &body, &crates, root) {
            let BodyViolation { file, kind, detail } = v;
            violations.push(Violation {
                file,
                kind: kind.to_string(),
                detail,
            });
        }
    }

    Ok(Report {
        adrs_checked: adrs.len(),
        crates_known: crates.len(),
        index_entries: index.len(),
        docs_scanned,
        violations,
    })
}

pub(crate) fn render_human(report: &Report) {
    println!(
        "Checked {} ADRs, {} crates known, {} index entries, {} markdown bodies.",
        report.adrs_checked, report.crates_known, report.index_entries, report.docs_scanned
    );
    if report.violations.is_empty() {
        println!("Coherence: ok (no violations).");
        return;
    }
    println!("Coherence: {} violation(s):", report.violations.len());
    for v in &report.violations {
        println!("  - [{}] {}: {}", v.kind, v.file, v.detail);
    }
}

pub(crate) fn render_plain(report: &Report) {
    println!(
        "checked={} crates_known={} index_entries={} docs_scanned={} violations={}",
        report.adrs_checked,
        report.crates_known,
        report.index_entries,
        report.docs_scanned,
        report.violations.len()
    );
}

pub(crate) fn statuses_match(file: &str, index: &str) -> bool {
    // Lenient: "superseded by 0042" in file vs "superseded" in index counts as a match.
    let f = file.trim().to_ascii_lowercase();
    let i = index.trim().to_ascii_lowercase();
    f == i || f.starts_with(&i) || i.starts_with(&f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statuses_match_lenient() {
        assert!(statuses_match("accepted", "accepted"));
        assert!(statuses_match("Accepted", "accepted"));
        assert!(statuses_match("superseded by 0042", "superseded"));
        assert!(!statuses_match("accepted", "proposed"));
    }
}
