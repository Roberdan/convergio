//! E2E coverage for GDPR data-subject-right routes.

mod common;

use reqwest::StatusCode;
use serde_json::json;

#[tokio::test]
async fn gdpr_request_fulfills_access_and_audits() {
    let (base, pool, _dir) = common::boot().await;
    let client = common::client();
    let res = client
        .post(format!("{base}/v1/gdpr/requests"))
        .json(&json!({
            "subject": "subj-1",
            "right": "access",
            "received_at": chrono::Utc::now(),
            "records": [{
                "record_id": "rec-1",
                "namespace": "ontology.object",
                "payload": {"value": "alpha"},
                "portable": true
            }],
            "agent_id": "agent-1"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["payload"]["article"], "15");
    assert_eq!(body["audit_seq"], 1);

    let audit_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM audit_log WHERE transition = 'gdpr.request.fulfilled'",
    )
    .fetch_one(pool.inner())
    .await
    .unwrap();
    assert_eq!(audit_count.0, 1);
}

#[tokio::test]
async fn gdpr_request_rejects_empty_subject() {
    let (base, _pool, _dir) = common::boot().await;
    let res = common::client()
        .post(format!("{base}/v1/gdpr/requests"))
        .json(&json!({
            "subject": " ",
            "right": "access",
            "received_at": chrono::Utc::now()
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
