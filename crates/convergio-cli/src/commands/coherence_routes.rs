//! `cvg coherence routes` — diff actual axum routes against documented ones.
//!
//! Walks every `*.rs` under `crates/convergio-server/src/routes/`,
//! parses `.route("/v1/...", get(...).post(...))` declarations into a
//! `(METHOD, PATH)` set, then parses the documented surface from:
//!
//! - `ARCHITECTURE.md` — the `### Endpoints` table.
//! - `AGENTS.md` — the HTTP routes bullet list.
//!
//! Reports three drift buckets:
//!
//! - `missing_in_docs` — route exists in code but no doc lists it.
//! - `missing_in_code` — doc lists a route nothing in code declares.
//! - `method_mismatch` — same path but methods diverge.
//!
//! `// docs-skip` on the same line as a `.route(...)` exempts that
//! route. Exit code is 0 on no drift, 1 otherwise. T1.18 / Tier-2
//! retrieval — companion to `cvg coherence check`.
//!
//! All user-facing strings flow through [`convergio_i18n::Bundle`] (P5).
//! Parsing helpers live in [`super::coherence_routes_parse`]; diffing
//! lives in [`super::coherence_routes_diff`] (300-line cap).

use super::coherence_routes_diff::{diff, split_methods_from_detail};
use super::coherence_routes_parse::{
    parse_agents_routes, parse_arch_endpoints, parse_code_routes, RouteEntry, Violation,
};
use super::OutputMode;
use anyhow::Result;
use convergio_i18n::Bundle;
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::Path;

/// Run the routes verifier against `root`.
pub async fn run(bundle: &Bundle, output: OutputMode, root: &Path) -> Result<()> {
    let report = run_check(root)?;
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&report)?),
        OutputMode::Plain => render_plain(&report),
        OutputMode::Human => render_human(bundle, &report),
    }
    if report.violations.is_empty() {
        Ok(())
    } else {
        std::process::exit(1)
    }
}

/// Full verifier report.
#[derive(Debug, Serialize)]
pub struct Report {
    /// Number of distinct (method, path) pairs in code.
    pub code_routes: usize,
    /// Number of distinct (method, path) pairs in docs.
    pub doc_routes: usize,
    /// Drift items.
    pub violations: Vec<Violation>,
}

fn run_check(root: &Path) -> Result<Report> {
    let code = parse_code_routes(&root.join("crates/convergio-server/src/routes"))?;
    let mut docs: Vec<RouteEntry> = Vec::new();
    let arch = root.join("ARCHITECTURE.md");
    if arch.exists() {
        docs.extend(parse_arch_endpoints(&arch)?);
    }
    let agents = root.join("AGENTS.md");
    if agents.exists() {
        docs.extend(parse_agents_routes(&agents)?);
    }
    let violations = diff(&code, &docs);
    Ok(Report {
        code_routes: distinct_pairs(&code),
        doc_routes: distinct_pairs(&docs),
        violations,
    })
}

fn distinct_pairs(entries: &[RouteEntry]) -> usize {
    let mut s: BTreeSet<(String, String)> = BTreeSet::new();
    for e in entries {
        for m in &e.methods {
            s.insert((m.clone(), e.path.clone()));
        }
    }
    s.len()
}

fn render_human(bundle: &Bundle, report: &Report) {
    let code = report.code_routes.to_string();
    let docs = report.doc_routes.to_string();
    let viol = report.violations.len().to_string();
    println!(
        "{}",
        bundle.t(
            "coherence-routes-summary",
            &[("code", &code), ("docs", &docs), ("violations", &viol)],
        )
    );
    if report.violations.is_empty() {
        println!("{}", bundle.t("coherence-routes-ok", &[]));
        return;
    }
    println!(
        "{}",
        bundle.t("coherence-routes-header", &[("count", &viol)])
    );
    for v in &report.violations {
        let line = match v.kind.as_str() {
            "missing_in_docs" => bundle.t(
                "coherence-routes-missing-in-docs",
                &[("method", &v.method), ("path", &v.path), ("file", &v.file)],
            ),
            "missing_in_code" => bundle.t(
                "coherence-routes-missing-in-code",
                &[("method", &v.method), ("path", &v.path), ("file", &v.file)],
            ),
            "method_mismatch" => {
                let (code_m, doc_m) = split_methods_from_detail(&v.detail);
                bundle.t(
                    "coherence-routes-method-mismatch",
                    &[
                        ("path", &v.path),
                        ("code_methods", &code_m),
                        ("doc_methods", &doc_m),
                    ],
                )
            }
            _ => v.detail.clone(),
        };
        println!("  - {line}");
    }
}

fn render_plain(report: &Report) {
    println!(
        "code_routes={} doc_routes={} violations={}",
        report.code_routes,
        report.doc_routes,
        report.violations.len()
    );
    for v in &report.violations {
        println!("{} {} {} {}", v.kind, v.method, v.path, v.file);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn end_to_end_synthetic_repo() {
        let dir = TempDir::new().expect("tempdir");
        let root = dir.path();
        let routes_dir = root.join("crates/convergio-server/src/routes");
        fs::create_dir_all(&routes_dir).expect("create routes dir");
        fs::write(
            routes_dir.join("health.rs"),
            "Router::new().route(\"/v1/health\", get(h));\n",
        )
        .expect("write health.rs");
        fs::write(
            routes_dir.join("plans.rs"),
            "Router::new().route(\"/v1/plans\", post(c).get(l));\n",
        )
        .expect("write plans.rs");
        fs::write(
            root.join("ARCHITECTURE.md"),
            concat!(
                "### Endpoints\n\n",
                "| Method | Path | Layer |\n",
                "|--------|------|-------|\n",
                "| GET | `/v1/health` | shell |\n",
                "| POST | `/v1/plans` | 1 |\n",
                "| GET | `/v1/audit` | 1 |\n",
                "\n## Next section\n",
            ),
        )
        .expect("write ARCHITECTURE.md");
        let report = run_check(root).expect("run_check");
        let kinds: Vec<&str> = report.violations.iter().map(|v| v.kind.as_str()).collect();
        assert!(kinds.contains(&"missing_in_code"));
        assert!(kinds.contains(&"method_mismatch"));
    }
}
