//! Unit tests for [`crate::adrs`].
//!
//! Split out to honour the 300-line per-file cap. Synthetic ADR text +
//! synthetic crate dirs cover each finding bucket. The bootstrapped
//! current-repo findings (ADR-0006/0007/0008/0023 etc.) are captured
//! as a fixture in [`bootstrap_findings_fixture`] so future drift
//! fixes do not silently break the test.

#![cfg(test)]

use crate::adrs::{run_check, Finding};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn write(p: &Path, body: &str) {
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(p, body).unwrap();
}

fn write_adr(root: &Path, id: &str, slug: &str, fm: &str) -> PathBuf {
    let p = root.join("docs/adr").join(format!("{id}-{slug}.md"));
    write(&p, &format!("---\n{fm}\n---\n# {id}\n"));
    p
}

#[test]
fn accepted_no_evidence_emits_when_crates_empty_of_match() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    write_adr(
        root,
        "0099",
        "totally-fictional-thing",
        "id: 0099\nstatus: accepted\ntouches_crates: [convergio-foo]",
    );
    fs::create_dir_all(root.join("crates/convergio-foo/src")).unwrap();
    write(
        &root.join("crates/convergio-foo/src/lib.rs"),
        "//! unrelated\nfn x() {}\n",
    );
    let report = run_check(root).unwrap();
    let row = report.rows.iter().find(|r| r.id == "0099").unwrap();
    assert_eq!(row.finding, Finding::AcceptedNoEvidence);
}

#[test]
fn accepted_with_adr_comment_passes() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    write_adr(
        root,
        "0098",
        "another-fictional",
        "id: 0098\nstatus: accepted\ntouches_crates: [convergio-bar]",
    );
    fs::create_dir_all(root.join("crates/convergio-bar/src")).unwrap();
    write(
        &root.join("crates/convergio-bar/src/lib.rs"),
        "// ADR-0098 implementation\nfn shipped() {}\n",
    );
    let report = run_check(root).unwrap();
    assert!(report.rows.iter().all(|r| r.id != "0098"));
}

#[test]
fn proposed_likely_shipped_emits_when_keyword_appears() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    write_adr(
        root,
        "0097",
        "foobars-everywhere",
        "id: 0097\nstatus: proposed\ntouches_crates: [convergio-baz]",
    );
    fs::create_dir_all(root.join("crates/convergio-baz/src")).unwrap();
    write(
        &root.join("crates/convergio-baz/src/lib.rs"),
        "//! foobars are shipped\nfn foobars() {}\n",
    );
    let report = run_check(root).unwrap();
    let row = report.rows.iter().find(|r| r.id == "0097").unwrap();
    assert_eq!(row.finding, Finding::ProposedLikelyShipped);
}

#[test]
fn broken_supersession_emits_when_target_missing() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    write_adr(
        root,
        "0096",
        "old",
        "id: 0096\nstatus: superseded by 9999\ntouches_crates: []",
    );
    let report = run_check(root).unwrap();
    let row = report.rows.iter().find(|r| r.id == "0096").unwrap();
    assert_eq!(row.finding, Finding::BrokenSupersession);
}

#[test]
fn supersession_passes_when_target_exists() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    write_adr(
        root,
        "0095",
        "old",
        "id: 0095\nstatus: superseded by 0094\ntouches_crates: []",
    );
    write_adr(
        root,
        "0094",
        "new",
        "id: 0094\nstatus: accepted\ntouches_crates: []",
    );
    let report = run_check(root).unwrap();
    assert!(report.rows.iter().all(|r| r.id != "0095"));
}

#[test]
fn finding_strict_classification() {
    assert!(Finding::AcceptedNoEvidence.is_strict());
    assert!(Finding::BrokenSupersession.is_strict());
    assert!(!Finding::ProposedLikelyShipped.is_strict());
}

/// Fixture: at the time of shipping, the verifier should flag at least
/// the four ADRs called out in PR #138 as `proposed_likely_shipped`.
/// If a future PR flips one of these to `accepted` (and the evidence
/// remains), this test will be the first to notice the test_run fixture
/// is stale and need updating.
const BOOTSTRAP_PROPOSED_SHIPPED: &[&str] = &["0006", "0007", "0008", "0023"];

#[test]
fn bootstrap_findings_fixture() {
    // Skip silently in environments where the workspace root cannot be
    // located (e.g. `cargo test` from an extracted source tarball).
    let manifest = match std::env::var("CARGO_MANIFEST_DIR") {
        Ok(m) => PathBuf::from(m),
        Err(_) => return,
    };
    // CARGO_MANIFEST_DIR points at crates/convergio-coherence/ — go up two.
    let workspace = manifest
        .parent()
        .and_then(|p| p.parent())
        .unwrap()
        .to_path_buf();
    if !workspace.join("docs/adr").exists() {
        return;
    }
    let report = run_check(&workspace).expect("run_check on workspace");
    for id in BOOTSTRAP_PROPOSED_SHIPPED {
        let row = report
            .rows
            .iter()
            .find(|r| r.id == *id)
            .unwrap_or_else(|| panic!("expected row for ADR-{id} in bootstrap fixture"));
        assert_eq!(
            row.finding,
            Finding::ProposedLikelyShipped,
            "ADR-{id} expected proposed_likely_shipped, got {:?}",
            row.finding
        );
    }
}
