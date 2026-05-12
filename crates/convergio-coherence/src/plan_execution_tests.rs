//! Unit tests for [`crate::plan_execution`].
//!
//! Split out of `plan_execution.rs` to honour the 300-line per-file
//! cap (CONSTITUTION § 13). Pure helpers (`infer_type`,
//! `required_kinds`) are exercised inline. `build_report` is tested
//! against a tiny in-process axum mock so the error-propagation
//! behavior covered by audit finding `plan_execution_scan.rs:79`
//! stays green.

#![cfg(test)]

use crate::plan_execution::{build_report, infer_type, required_kinds, TaskType};
use axum::{routing::get, Json, Router};
use std::collections::HashSet;
use tokio::net::TcpListener;

async fn spawn(router: Router) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    format!("http://{addr}")
}

#[test]
fn infer_type_code_from_code_evidence() {
    let mut kinds = HashSet::new();
    kinds.insert("code".to_string());
    kinds.insert("context_pack".to_string());
    assert_eq!(infer_type(&kinds), TaskType::Code);
}

#[test]
fn infer_type_code_from_merge_record() {
    let mut kinds = HashSet::new();
    kinds.insert("merge_record".to_string());
    assert_eq!(infer_type(&kinds), TaskType::Code);
}

#[test]
fn infer_type_doc_only() {
    let mut kinds = HashSet::new();
    kinds.insert("adr".to_string());
    assert_eq!(infer_type(&kinds), TaskType::DocOnly);
}

#[test]
fn infer_type_analysis_when_empty() {
    let kinds = HashSet::new();
    assert_eq!(infer_type(&kinds), TaskType::Analysis);
}

#[test]
fn code_task_requires_graph_and_ci() {
    let required = required_kinds(&TaskType::Code);
    assert!(required.contains(&"context_pack"));
    assert!(required.contains(&"ci_run"));
    assert!(required.contains(&"merge_record"));
}

#[test]
fn analysis_has_no_requirements() {
    assert!(required_kinds(&TaskType::Analysis).is_empty());
}

// Regression test for audit finding `plan_execution_scan.rs:79`:
// evidence fetch failures were swallowed into an empty vec, so
// strict reports looked like missing evidence instead of a
// transport/decode failure. With the fix `build_report` surfaces
// the error to the caller.
#[tokio::test]
async fn build_report_propagates_evidence_fetch_failure() {
    let router = Router::new()
        .route(
            "/v1/plans/:plan_id/tasks",
            get(|| async {
                Json(serde_json::json!([
                    { "id": "task-1", "title": "x", "status": "done" }
                ]))
            }),
        )
        .route(
            "/v1/tasks/:id/evidence",
            get(|| async { (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "boom") }),
        )
        .route(
            "/v1/agent-registry/agents",
            get(|| async { Json(serde_json::json!([])) }),
        )
        .route(
            "/v1/plans/:plan_id/messages",
            get(|| async { Json(serde_json::json!([])) }),
        );
    let base = spawn(router).await;
    let client = reqwest::Client::new();
    let res = build_report(&client, &base, "plan-1").await;
    assert!(
        res.is_err(),
        "expected build_report to surface evidence fetch HTTP 500 as Err, got {res:?}"
    );
}
