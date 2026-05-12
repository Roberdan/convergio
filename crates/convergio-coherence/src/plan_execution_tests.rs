//! Reproduction tests for [`crate::plan_execution`] daemon-side
//! behaviors that the inline `mod tests` cannot easily host (they
//! need a tokio runtime + an in-process axum mock). Keeping these
//! beside the module so the audit-finding traces stay close to the
//! verifier code (CONSTITUTION § 5, P5).

#![cfg(test)]

use crate::plan_execution::build_report;
use axum::{routing::get, Json, Router};
use tokio::net::TcpListener;

async fn spawn(router: Router) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    format!("http://{addr}")
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
