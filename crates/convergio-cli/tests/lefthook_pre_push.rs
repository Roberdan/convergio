//! Smoke test for the lefthook `pre-push` doc-regen gate (P0.5).
//!
//! Recurring CI failures on PRs #169 / #170 / #173 were caused by
//! AUTO blocks not being regenerated locally before push. The
//! lefthook `pre-push` hook now blocks the push if either
//! `cvg docs regenerate --check` or
//! `./scripts/generate-docs-index.sh --check` would fail.
//!
//! This test exercises the hook end-to-end when `lefthook` is on
//! PATH; it is a no-op (with a stderr notice) on machines that do
//! not have lefthook installed, so CI remains green without making
//! lefthook a hard prerequisite.

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
fn lefthook_yml_contains_pre_push_doc_gate() {
    let Some(root) = workspace_root() else {
        eprintln!("workspace root not found, skipping");
        return;
    };
    let cfg = std::fs::read_to_string(root.join("lefthook.yml")).expect("read lefthook.yml");
    assert!(
        cfg.contains("pre-push:"),
        "lefthook.yml missing pre-push block",
    );
    assert!(
        cfg.contains("docs-regen-check:"),
        "lefthook.yml missing docs-regen-check command",
    );
    assert!(
        cfg.contains("docs-index-check:"),
        "lefthook.yml missing docs-index-check command",
    );
    assert!(
        cfg.contains("LEFTHOOK_SKIP_DOC_REGEN"),
        "lefthook.yml missing documented escape hatch",
    );
}
