//! E2E for `/v1/purposes` — Purpose Registry (ADR-0054 §B).
//!
//! Drives the daemon HTTP surface: register a purpose, list it back, and
//! confirm that re-declaring an existing label is refused (purposes are
//! immutable).

mod common;

use axum::http::StatusCode;
use common::{boot, client};
use serde_json::{json, Value};

#[tokio::test]
async fn register_list_and_reject_duplicate() {
    let (base, _pool, _dir) = boot().await;
    let http = client();

    let resp = http
        .post(format!("{base}/v1/purposes"))
        .json(&json!({"label": "student-records", "description": "Manage student records"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["label"], "student-records");
    assert!(!body["id"].as_str().unwrap().is_empty());
    assert!(!body["effective_from"].as_str().unwrap().is_empty());

    let listed: Value = http
        .get(format!("{base}/v1/purposes"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(listed.as_array().unwrap().len(), 1);
    assert_eq!(listed[0]["label"], "student-records");

    let dup = http
        .post(format!("{base}/v1/purposes"))
        .json(&json!({"label": "student-records"}))
        .send()
        .await
        .unwrap();
    assert_eq!(dup.status(), StatusCode::BAD_REQUEST);
    let err: Value = dup.json().await.unwrap();
    assert_eq!(err["error"]["code"], "purpose_already_exists");
}

#[tokio::test]
async fn empty_label_is_rejected() {
    let (base, _pool, _dir) = boot().await;
    let http = client();

    let resp = http
        .post(format!("{base}/v1/purposes"))
        .json(&json!({"label": "   "}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let err: Value = resp.json().await.unwrap();
    assert_eq!(err["error"]["code"], "invalid_purpose");
}
