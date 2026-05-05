//! Smoke test asserting the lefthook `pre-push` doc-regen gate is
//! gone (retro 544e78cc P2-9 / ADR-0015 § Drift policy).
//!
//! The gate added in P0.5 (#178) was a band-aid for the recurring
//! CI failures on PRs #169 / #170 / #173 caused by per-PR AUTO-block
//! regen. The root-cause fix is to stop regenerating per-PR (cron
//! reconciles nightly) — so the lefthook commands `docs-regen-check`
//! and `docs-index-check` MUST NOT be present in `lefthook.yml`.
//!
//! This test also runs `lefthook validate` when the binary is on
//! PATH to catch syntactic regressions; it is a no-op (with a stderr
//! notice) on machines that do not have lefthook installed, so CI
//! remains green without making lefthook a hard prerequisite.

use std::path::PathBuf;
use std::process::Command;

/// Walk up from this crate's `CARGO_MANIFEST_DIR` until we find the
/// workspace root (contains a `lefthook.yml`). Returns `None` if no
/// such ancestor exists — should never happen inside this repo.
fn workspace_root() -> Option<PathBuf> {
    let start = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut cur: &std::path::Path = &start;
    loop {
        if cur.join("lefthook.yml").is_file() {
            return Some(cur.to_path_buf());
        }
        match cur.parent() {
            Some(p) => cur = p,
            None => return None,
        }
    }
}

#[test]
fn lefthook_pre_push_runs_when_available() {
    if Command::new("lefthook").arg("--version").output().is_err() {
        eprintln!("lefthook not installed, skipping");
        return;
    }
    let Some(root) = workspace_root() else {
        eprintln!("workspace root not found, skipping");
        return;
    };
    // `lefthook validate` is the cheapest call that proves the
    // pre-push block we just wrote parses cleanly. The hook itself
    // would rebuild the workspace, which is too heavy for a unit
    // smoke test.
    let out = Command::new("lefthook")
        .arg("validate")
        .current_dir(&root)
        .output()
        .expect("lefthook validate runnable");
    assert!(
        out.status.success(),
        "lefthook validate failed: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

#[test]
fn lefthook_yml_has_no_pre_push_doc_gate() {
    // ADR-0015 § Drift policy. The per-PR doc-regen gate is removed;
    // drift is reconciled by the nightly cron in
    // `.github/workflows/auto-blocks-drift.yml`. Re-introducing
    // either gate brings back the O(N²) merge cascade.
    let Some(root) = workspace_root() else {
        eprintln!("workspace root not found, skipping");
        return;
    };
    let cfg = std::fs::read_to_string(root.join("lefthook.yml")).expect("read lefthook.yml");
    assert!(
        !cfg.contains("docs-regen-check:"),
        "lefthook.yml must NOT contain docs-regen-check (ADR-0015 § Drift policy)",
    );
    assert!(
        !cfg.contains("docs-index-check:"),
        "lefthook.yml must NOT contain docs-index-check (ADR-0015 § Drift policy)",
    );
    assert!(
        !cfg.contains("LEFTHOOK_SKIP_DOC_REGEN"),
        "lefthook.yml must NOT reference the doc-regen escape hatch (ADR-0015 § Drift policy)",
    );
}
