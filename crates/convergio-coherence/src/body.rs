//! Body-text drift detector for [`crate::coherence`].
//!
//! Walks every `*.md` under the repo root and looks for two kinds
//! of unresolved references inside live (non-fenced) text:
//!
//! - `convergio-foo` style identifiers — must be in
//!   `workspace.members` (or in [`ALLOW_IDENTS`]).
//! - File paths under `crates|docs|scripts|examples/` — must exist
//!   on disk.
//!
//! Line-level scanning lives in [`crate::body_scan`].

use crate::body_scan::{find_crate_idents, find_repo_paths, live_lines};
use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::path::Path;

/// One unresolved body-text reference.
#[derive(Debug, Clone)]
pub(super) struct BodyViolation {
    pub(super) file: String,
    pub(super) kind: &'static str,
    pub(super) detail: String,
}

/// Scan one markdown body. `crates` is the workspace.members set;
/// `repo_root` is used to verify path references.
pub(super) fn scan_body(
    rel_file: &str,
    body: &str,
    crates: &BTreeSet<String>,
    repo_root: &Path,
) -> Vec<BodyViolation> {
    if SKIP_FILES.iter().any(|f| rel_file.ends_with(f)) {
        return Vec::new();
    }
    let mut out: Vec<BodyViolation> = Vec::new();
    let mut seen_crates: BTreeSet<String> = BTreeSet::new();
    let mut seen_paths: BTreeSet<String> = BTreeSet::new();

    for line in live_lines(body) {
        for ident in find_crate_idents(line) {
            if crates.contains(&ident) || ALLOW_IDENTS.contains(&ident.as_str()) {
                continue;
            }
            if seen_crates.insert(ident.clone()) {
                out.push(BodyViolation {
                    file: rel_file.to_string(),
                    kind: "unknown_crate_reference",
                    detail: format!("body mentions '{ident}' which is not in workspace.members"),
                });
            }
        }
        for path in find_repo_paths(line) {
            if path.chars().any(|c| c.is_ascii_uppercase()) {
                continue;
            }
            if seen_paths.contains(&path) {
                continue;
            }
            seen_paths.insert(path.clone());
            let stripped = path
                .trim_end_matches([')', ']', '.', ',', ';', ':', '!', '?'])
                .to_string();
            if repo_root.join(&stripped).exists() {
                continue;
            }
            if stripped.contains('*') {
                continue;
            }
            out.push(BodyViolation {
                file: rel_file.to_string(),
                kind: "missing_path_reference",
                detail: format!("body references '{stripped}' which does not exist on disk"),
            });
        }
    }
    out
}

/// Walk all `*.md` under root, returning (rel_path, contents).
pub(super) fn walk_markdown(root: &Path) -> Result<Vec<(String, String)>> {
    let mut out: Vec<(String, String)> = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        if path.components().any(|c| {
            matches!(
                c.as_os_str().to_str(),
                Some("target") | Some(".git") | Some(".claude")
            )
        }) {
            continue;
        }
        let rel = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned();
        let body = std::fs::read_to_string(path).with_context(|| format!("read {rel}"))?;
        out.push((rel, body));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

/// Markdown files we skip entirely. CHANGELOG is auto-generated and
/// references retired crates historically; the ADR template uses
/// placeholder text by design; LICENSE / SECURITY are non-technical.
const SKIP_FILES: &[&str] = &[
    "CHANGELOG.md",
    "LICENSE",
    "SECURITY.md",
    "docs/adr/0000-template.md",
    "0000-template.md",
];

/// `convergio-X` identifiers that are NOT workspace crates but are
/// legitimately mentioned in docs:
///   1. Repo / binary group names (`convergio-local`).
///   2. Retired crates (`convergio-worktree`) kept in historical text.
///   3. Future / proposed crates from open ADRs (`convergio-audit`,
///      `convergio-state`, `convergio-coordination` from ADR-0013;
///      `convergio-acp`, `convergio-cap` from forward-looking ADRs).
///   4. Release artefact names (`convergio-darwin-arm64`, etc.).
const ALLOW_IDENTS: &[&str] = &[
    "convergio-local",
    "convergio-local-public-readiness",
    "convergio-worktree",
    "convergio-feature",
    "convergio-audit",
    "convergio-state",
    "convergio-coordination",
    "convergio-acp",
    "convergio-cap",
    "convergio-migrations",
    "convergio-notary",
    "convergio-darwin-arm64",
    "convergio-darwin-arm64-signed",
    // Future vertical accelerators referenced in ROADMAP / Wave 0
    // ADRs (ADR-0016 long-tail, ADR-0018 urbanism, ADR-0019
    // thinking-stack, ADR-0021 OKR-on-plans). Not in
    // workspace.members today; will be when each vertical ships.
    "convergio-edu",
    "convergio-edu-v1",
    "convergio-research",
    "convergio-thinking-bundles",
];

#[cfg(test)]
mod tests {
    use super::*;

    fn cratest() -> BTreeSet<String> {
        ["convergio-cli", "convergio-graph"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    #[test]
    fn flags_unknown_crate_reference() {
        let v = scan_body(
            "AGENTS.md",
            "See `convergio-doesnotexist` for details.",
            &cratest(),
            Path::new("."),
        );
        assert!(v.iter().any(|x| x.kind == "unknown_crate_reference"));
    }

    #[test]
    fn known_crate_passes() {
        let v = scan_body(
            "AGENTS.md",
            "Run `convergio-cli` and `convergio-graph`.",
            &cratest(),
            Path::new("."),
        );
        assert!(v.is_empty());
    }

    #[test]
    fn skips_fenced_examples() {
        let body = "before\n```\nconvergio-doesnotexist-fenced\n```\nafter\n";
        let v = scan_body("doc.md", body, &cratest(), Path::new("."));
        assert!(v.is_empty());
    }

    #[test]
    fn dedups_within_one_file() {
        let body = "convergio-doesnotexist and again convergio-doesnotexist\n";
        let v = scan_body("doc.md", body, &cratest(), Path::new("."));
        let unknowns: Vec<&BodyViolation> = v
            .iter()
            .filter(|x| x.kind == "unknown_crate_reference")
            .collect();
        assert_eq!(unknowns.len(), 1);
    }

    #[test]
    fn skip_files_returns_empty() {
        let v = scan_body(
            "CHANGELOG.md",
            "convergio-doesnotexist is not in members",
            &cratest(),
            Path::new("."),
        );
        assert!(v.is_empty());
    }

    #[test]
    fn allowlist_prevents_flag() {
        let v = scan_body(
            "AGENTS.md",
            "convergio-audit will be split out (ADR-0013)",
            &cratest(),
            Path::new("."),
        );
        assert!(v.is_empty());
    }
}
