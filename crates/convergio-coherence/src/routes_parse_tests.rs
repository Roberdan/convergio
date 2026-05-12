//! Unit tests for [`crate::routes_parse`].
//!
//! Split out of `routes_parse.rs` to honour the 300-line per-file cap
//! (CONSTITUTION § 13).

#![cfg(test)]

use crate::routes_parse::{extract_methods, parse_doc_methods, parse_route_lines};

#[test]
fn extracts_methods_from_chain() {
    let m = extract_methods(", get(handler).post(other))");
    assert!(m.contains("GET"));
    assert!(m.contains("POST"));
}

#[test]
fn parse_route_lines_basic() {
    let body = r#"
    Router::new()
        .route("/v1/health", get(health))
        .route("/v1/plans", post(create).get(list))
        .route("/v1/internal", get(internal)) // docs-skip
    "#;
    let r = parse_route_lines(body, "routes/x.rs");
    assert_eq!(r.len(), 2);
    let plans = r.iter().find(|e| e.path == "/v1/plans").expect("plans");
    assert!(plans.methods.contains("POST"));
    assert!(plans.methods.contains("GET"));
}

#[test]
fn doc_methods_handles_dot_separator() {
    let m = parse_doc_methods("POST · GET");
    assert!(m.contains("POST"));
    assert!(m.contains("GET"));
}

#[test]
fn docs_skip_exempts_route() {
    let body = "        .route(\"/v1/internal/debug\", get(debug)) // docs-skip\n";
    let r = parse_route_lines(body, "routes/x.rs");
    assert!(r.is_empty());
}
