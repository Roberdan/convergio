//! E2E coverage for dispatch executor modes.

mod common;

use reqwest::StatusCode;
use serde_json::json;

#[tokio::test]
async fn dispatch_executor_none_is_tracker_only() {
    let (base, _pool, _dir) = common::boot().await;
    let res = common::client()
        .post(format!("{base}/v1/dispatch"))
        .json(&json!({"executor":"none"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["dispatched"], 0);
    assert_eq!(body["tracker_only"], true);
}

#[tokio::test]
async fn fleet_repo_dispatch_accepts_registered_repo() {
    let (base, _pool, _dir) = common::boot().await;
    let client = common::client();
    let repo = std::env::current_dir().unwrap();
    let add = client
        .post(format!("{base}/v1/fleet/repos"))
        .json(&json!({
            "name": "engine",
            "path": repo,
            "language": "rust",
            "parser": "syn",
            "role": "engine"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(add.status(), StatusCode::OK);
    let res = client
        .post(format!("{base}/v1/dispatch"))
        .json(&json!({"repo":"engine","executor":"none"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}
