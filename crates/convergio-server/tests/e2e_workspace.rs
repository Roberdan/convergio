//! Workspace lease API E2E tests.

use chrono::{Duration, Utc};
use convergio_bus::Bus;
use convergio_db::Pool;
use convergio_durability::init;
use convergio_lifecycle::Supervisor;
use convergio_server::{router, AppState};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::net::TcpListener;

async fn boot() -> (String, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("state.db");
    let url = format!("sqlite://{}", db_path.display());
    let pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    convergio_bus::init(&pool).await.unwrap();
    convergio_lifecycle::init(&pool).await.unwrap();

    let state = AppState {
        durability: Arc::new(convergio_durability::Durability::new(pool.clone())),
        bus: Arc::new(Bus::new(pool.clone())),
        supervisor: Arc::new(Supervisor::new(pool.clone())),
        graph: Arc::new(convergio_graph::Store::new(pool.clone())),
        embed: Arc::new(convergio_embed::EmbedStore::new(pool.clone())),
    };
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router(state)).await.unwrap();
    });
    (format!("http://{addr}"), dir)
}

fn claim(agent_id: &str) -> Value {
    json!({
        "resource": {
            "kind": "file",
            "path": "src/lib.rs"
        },
        "task_id": "task-lease",
        "agent_id": agent_id,
        "purpose": "edit",
        "expires_at": (Utc::now() + Duration::minutes(10)).to_rfc3339()
    })
}

fn patch(agent_id: &str) -> Value {
    json!({
        "task_id": "task-lease",
        "agent_id": agent_id,
        "base_revision": "base",
        "patch": "diff --git",
        "files": [{
            "path": "src/lib.rs",
            "project": null,
            "base_hash": "same",
            "current_hash": "same",
            "proposed_hash": "next"
        }]
    })
}

#[tokio::test]
async fn workspace_lease_api_refuses_overlap_until_release() {
    let (base, _dir) = boot().await;
    let client = reqwest::Client::new();

    let first: Value = client
        .post(format!("{base}/v1/workspace/leases"))
        .json(&claim("agent-a"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let lease_id = first["id"].as_str().unwrap();
    assert_eq!(first["status"], "active");

    let resp = client
        .post(format!("{base}/v1/workspace/leases"))
        .json(&claim("agent-b"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 409);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"]["code"], "workspace_lease_conflict");

    let active: Value = client
        .get(format!("{base}/v1/workspace/leases"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(active.as_array().unwrap().len(), 1);

    let released: Value = client
        .post(format!("{base}/v1/workspace/leases/{lease_id}/release"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(released["status"], "released");

    let active: Value = client
        .get(format!("{base}/v1/workspace/leases"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(active.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn patch_proposals_require_lease_coverage_and_record_conflicts() {
    let (base, _dir) = boot().await;
    let client = reqwest::Client::new();

    let _: Value = client
        .post(format!("{base}/v1/workspace/leases"))
        .json(&claim("agent-a"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let proposal: Value = client
        .post(format!("{base}/v1/workspace/patches"))
        .json(&patch("agent-a"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(proposal["status"], "proposed");

    let resp = client
        .post(format!("{base}/v1/workspace/patches"))
        .json(&patch("agent-b"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 409);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"]["code"], "workspace_patch_refused");

    let conflicts: Value = client
        .get(format!("{base}/v1/workspace/conflicts"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(conflicts.as_array().unwrap().len(), 1);
    assert_eq!(conflicts[0]["kind"], "lease_conflict");

    let audit: Value = client
        .get(format!("{base}/v1/audit/verify"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(audit["ok"], true);
}
