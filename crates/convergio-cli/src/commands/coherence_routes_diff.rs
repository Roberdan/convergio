//! Diff for [`super::coherence_routes`].
//!
//! Builds three drift buckets — `missing_in_docs`, `missing_in_code`,
//! `method_mismatch` — from a code route set and a docs route set.

use super::coherence_routes_parse::{RouteEntry, Violation};
use std::collections::{BTreeMap, BTreeSet};

fn pairs(entries: &[RouteEntry]) -> BTreeSet<(String, String)> {
    let mut out: BTreeSet<(String, String)> = BTreeSet::new();
    for e in entries {
        for m in &e.methods {
            out.insert((m.clone(), e.path.clone()));
        }
    }
    out
}

fn by_path(entries: &[RouteEntry]) -> BTreeMap<String, (BTreeSet<String>, String)> {
    let mut out: BTreeMap<String, (BTreeSet<String>, String)> = BTreeMap::new();
    for e in entries {
        let slot = out
            .entry(e.path.clone())
            .or_insert_with(|| (BTreeSet::new(), e.file.clone()));
        for m in &e.methods {
            slot.0.insert(m.clone());
        }
    }
    out
}

/// Compute drift across the three buckets.
pub(super) fn diff(code: &[RouteEntry], docs: &[RouteEntry]) -> Vec<Violation> {
    let code_pairs = pairs(code);
    let doc_pairs = pairs(docs);
    let code_by_path = by_path(code);
    let doc_by_path = by_path(docs);

    let mut out: Vec<Violation> = Vec::new();
    for (m, p) in code_pairs.difference(&doc_pairs) {
        if !doc_by_path.contains_key(p) {
            let file = code_by_path
                .get(p)
                .map(|(_, f)| f.clone())
                .unwrap_or_default();
            out.push(Violation {
                kind: "missing_in_docs".into(),
                path: p.clone(),
                method: m.clone(),
                file,
                detail: format!(
                    "{m} {p} declared in code but absent from ARCHITECTURE.md / AGENTS.md"
                ),
            });
        }
    }
    for (m, p) in doc_pairs.difference(&code_pairs) {
        if !code_by_path.contains_key(p) {
            let file = doc_by_path
                .get(p)
                .map(|(_, f)| f.clone())
                .unwrap_or_default();
            out.push(Violation {
                kind: "missing_in_code".into(),
                path: p.clone(),
                method: m.clone(),
                file,
                detail: format!("{m} {p} documented but no axum route declares it"),
            });
        }
    }
    for (p, (code_methods, code_file)) in &code_by_path {
        if let Some((doc_methods, _)) = doc_by_path.get(p) {
            if code_methods != doc_methods {
                let cs = code_methods.iter().cloned().collect::<Vec<_>>().join(",");
                let ds = doc_methods.iter().cloned().collect::<Vec<_>>().join(",");
                out.push(Violation {
                    kind: "method_mismatch".into(),
                    path: p.clone(),
                    method: String::new(),
                    file: code_file.clone(),
                    detail: format!("{p}: code methods [{cs}] differ from doc methods [{ds}]"),
                });
            }
        }
    }
    out.sort_by(|a, b| a.kind.cmp(&b.kind).then_with(|| a.path.cmp(&b.path)));
    out
}

/// Pull the `[code]` and `[doc]` method sets out of a `method_mismatch` detail.
pub(super) fn split_methods_from_detail(detail: &str) -> (String, String) {
    let code = between(detail, "code methods [", "]").unwrap_or_default();
    let doc = between(detail, "doc methods [", "]").unwrap_or_default();
    (code, doc)
}

fn between(s: &str, start: &str, end: &str) -> Option<String> {
    let i = s.find(start)? + start.len();
    let rest = &s[i..];
    let j = rest.find(end)?;
    Some(rest[..j].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn route(method: &str, path: &str, file: &str) -> RouteEntry {
        let mut methods = BTreeSet::new();
        methods.insert(method.to_string());
        RouteEntry {
            path: path.to_string(),
            methods,
            file: file.to_string(),
        }
    }

    #[test]
    fn diff_buckets() {
        let code = vec![
            route("GET", "/v1/health", "routes/health.rs"),
            route("POST", "/v1/plans", "routes/plans.rs"),
        ];
        let docs = vec![
            route("GET", "/v1/health", "ARCHITECTURE.md"),
            route("GET", "/v1/audit", "ARCHITECTURE.md"),
        ];
        let v = diff(&code, &docs);
        let kinds: BTreeSet<&str> = v.iter().map(|x| x.kind.as_str()).collect();
        assert!(kinds.contains("missing_in_docs"));
        assert!(kinds.contains("missing_in_code"));
        assert!(v.iter().any(|x| x.path == "/v1/plans"));
        assert!(v.iter().any(|x| x.path == "/v1/audit"));
    }

    #[test]
    fn diff_method_mismatch() {
        let mut methods = BTreeSet::new();
        methods.insert("GET".to_string());
        methods.insert("POST".to_string());
        let code = vec![RouteEntry {
            path: "/v1/plans".into(),
            methods,
            file: "routes/plans.rs".into(),
        }];
        let docs = vec![route("GET", "/v1/plans", "ARCHITECTURE.md")];
        let v = diff(&code, &docs);
        assert!(v.iter().any(|x| x.kind == "method_mismatch"));
    }

    #[test]
    fn split_methods_extracts_pair() {
        let d = "/v1/x: code methods [GET,POST] differ from doc methods [GET]";
        let (c, doc) = split_methods_from_detail(d);
        assert_eq!(c, "GET,POST");
        assert_eq!(doc, "GET");
    }
}
