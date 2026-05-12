//! Route parsers for [`crate::routes`].
//!
//! Split out to honour the 300-line per-file cap. Diff logic lives
//! in [`crate::routes_diff`].

use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// One axum route discovered in code or docs.
#[derive(Debug, Clone)]
pub(super) struct RouteEntry {
    pub(super) path: String,
    pub(super) methods: BTreeSet<String>,
    pub(super) file: String,
}

/// One drift item the verifier found.
#[derive(Debug, Clone, Serialize)]
pub(super) struct Violation {
    pub(super) kind: String,
    pub(super) path: String,
    pub(super) method: String,
    pub(super) file: String,
    pub(super) detail: String,
}

/// Walk the routes directory and parse every `.route(...)` line.
pub(super) fn parse_code_routes(dir: &Path) -> Result<Vec<RouteEntry>> {
    let mut out: Vec<RouteEntry> = Vec::new();
    if !dir.exists() {
        return Ok(out);
    }
    let mut files: Vec<PathBuf> = Vec::new();
    for entry in std::fs::read_dir(dir).with_context(|| format!("read_dir {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rs") {
            files.push(path);
        }
    }
    files.sort();
    for path in files {
        let body =
            std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let rel = relativize(&path);
        for entry in parse_route_lines(&body, &rel) {
            out.push(entry);
        }
    }
    Ok(out)
}

/// Parse `.route("/v1/...", METHOD_CHAIN)` lines from one file body.
/// `// docs-skip` on the same line exempts the route.
pub(super) fn parse_route_lines(body: &str, file: &str) -> Vec<RouteEntry> {
    let mut out: Vec<RouteEntry> = Vec::new();
    for raw in body.lines() {
        let Some(idx) = raw.find(".route(") else {
            continue;
        };
        if raw.contains("// docs-skip") {
            continue;
        }
        let line = &raw[idx..];
        let after_quote = match line.find('"') {
            Some(i) => &line[i + 1..],
            None => continue,
        };
        let close = match after_quote.find('"') {
            Some(i) => i,
            None => continue,
        };
        let path = after_quote[..close].to_string();
        let methods = extract_methods(&after_quote[close + 1..]);
        if methods.is_empty() {
            continue;
        }
        out.push(RouteEntry {
            path,
            methods,
            file: file.to_string(),
        });
    }
    out
}

pub(crate) fn extract_methods(rest: &str) -> BTreeSet<String> {
    const VERBS: &[&str] = &["get", "post", "put", "patch", "delete", "head", "options"];
    let mut out: BTreeSet<String> = BTreeSet::new();
    for verb in VERBS {
        let needle = format!("{verb}(");
        for (i, _) in rest.match_indices(&needle) {
            let prev = if i == 0 {
                None
            } else {
                rest.as_bytes().get(i - 1).copied()
            };
            if matches!(prev, Some(b) if b.is_ascii_alphanumeric() || b == b'_') {
                continue;
            }
            out.insert(verb.to_ascii_uppercase());
        }
    }
    out
}

/// Parse the `### Endpoints` Markdown table in ARCHITECTURE.md.
pub(super) fn parse_arch_endpoints(path: &Path) -> Result<Vec<RouteEntry>> {
    let body = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let rel = relativize(path);
    let mut out: Vec<RouteEntry> = Vec::new();
    let mut in_table = false;
    for line in body.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("### Endpoints") {
            in_table = true;
            continue;
        }
        if in_table && trimmed.starts_with("## ") {
            break;
        }
        if !in_table || !trimmed.starts_with('|') {
            continue;
        }
        let cells: Vec<&str> = trimmed.split('|').map(str::trim).collect();
        if cells.len() < 4 {
            continue;
        }
        let methods_cell = cells[1];
        let path_cell = cells[2];
        if methods_cell.eq_ignore_ascii_case("Method") || methods_cell.starts_with("---") {
            continue;
        }
        let methods = parse_doc_methods(methods_cell);
        if methods.is_empty() {
            continue;
        }
        for p in extract_doc_paths(path_cell) {
            out.push(RouteEntry {
                path: p,
                methods: methods.clone(),
                file: rel.clone(),
            });
        }
    }
    Ok(out)
}

/// Parse the HTTP routes bullet list in `AGENTS.md`. Picks every
/// backticked `METHOD /v1/...` token.
pub(super) fn parse_agents_routes(path: &Path) -> Result<Vec<RouteEntry>> {
    let body = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let rel = relativize(path);
    let mut out: Vec<RouteEntry> = Vec::new();
    for token in extract_backtick_tokens(&body) {
        let Some((mlist, p)) = token.split_once(' ') else {
            continue;
        };
        let p = p.trim();
        if !p.starts_with("/v1/") {
            continue;
        }
        // Drop a query-string suffix the docs sometimes attach for
        // illustration (`/v1/x?topic=&cursor=`); axum sees only the
        // path so the suffix is non-load-bearing here.
        let p = p.split('?').next().unwrap_or(p);
        let methods = parse_doc_methods(mlist);
        if methods.is_empty() {
            continue;
        }
        out.push(RouteEntry {
            path: p.to_string(),
            methods,
            file: rel.clone(),
        });
    }
    Ok(out)
}

fn extract_backtick_tokens(body: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let bytes = body.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'`' {
            if let Some(end) = body[i + 1..].find('`') {
                let token = &body[i + 1..i + 1 + end];
                if !token.is_empty() {
                    out.push(token.to_string());
                }
                i += end + 2;
                continue;
            }
        }
        i += 1;
    }
    out
}

pub(crate) fn parse_doc_methods(cell: &str) -> BTreeSet<String> {
    const VERBS: &[&str] = &["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];
    let upper = cell.to_ascii_uppercase();
    let mut out: BTreeSet<String> = BTreeSet::new();
    for v in VERBS {
        for (i, _) in upper.match_indices(v) {
            let before = if i == 0 {
                None
            } else {
                upper.as_bytes().get(i - 1).copied()
            };
            let after = upper.as_bytes().get(i + v.len()).copied();
            let bounded_before = !matches!(before, Some(b) if b.is_ascii_alphabetic());
            let bounded_after = !matches!(after, Some(b) if b.is_ascii_alphabetic());
            if bounded_before && bounded_after {
                out.insert((*v).to_string());
            }
        }
    }
    out
}

fn extract_doc_paths(cell: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for token in extract_backtick_tokens(cell) {
        if token.starts_with("/v1/") {
            out.push(token);
        }
    }
    out
}

fn relativize(path: &Path) -> String {
    let s = path.to_string_lossy().to_string();
    s.trim_start_matches("./").to_string()
}
