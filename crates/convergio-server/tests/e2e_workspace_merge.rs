//! Workspace merge queue E2E tests.

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
    let pool = Pool::connect(&format!("sqlite://{}", db_path.display()))
        .await
        .unwrap();
    init(&pool).await.unwrap();
    convergio_bus::init(&pool).await.unwrap();
    convergio_lifecycle::init(&pool).await.unwrap();
    let state = AppState {
        durability: Arc::new(convergio_durability::Durability::new(pool.clone())),
        ops: Arc::new(convergio_ops::Ops::new(pool.clone())),
        bus: Arc::new(Bus::new(pool.clone())),
        supervisor: Arc::new(Supervisor::new(pool.clone())),
        graph: Arc::new(convergio_graph::Store::new(pool.clone())),
        embed: Arc::new(convergio_embed::EmbedStore::new(pool.clone())),
        embedder: Arc::new(convergio_embed::embedder::testing::DeterministicTestEmbedder::new(8)),
        fleet: Arc::new(convergio_fleet::FleetStore::new(pool.clone())),
        fleet_plans: Arc::new(convergio_fleet::FleetPlanStore::new(pool.clone())),
        ontology: Arc::new(convergio_ontology::Store::new(pool.clone())),
        audit_verify_cache: Arc::new(std::sync::Mutex::new(None)),
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

fn claim(agent_id: &str, path: &str) -> Value {
    json!({
        "resource": {"kind": "file", "path": path},
        "task_id": "task-lease",
        "agent_id": agent_id,
        "purpose": "edit",
        "expires_at": (Utc::now() + Duration::minutes(10)).to_rfc3339()
    })
}

fn patch(agent_id: &str, path: &str, proposed_hash: &str) -> Value {
    patch_hashes(agent_id, path, "same", "same", proposed_hash)
}

fn patch_hashes(
    agent_id: &str,
    path: &str,
    base_hash: &str,
    current_hash: &str,
    proposed_hash: &str,
) -> Value {
    json!({
        "task_id": "task-lease",
        "agent_id": agent_id,
        "base_revision": "base",
        "patch": "diff --git",
        "files": [{
            "path": path,
            "project": null,
            "base_hash": base_hash,
            "current_hash": current_hash,
            "proposed_hash": proposed_hash
        }]
    })
}

async fn submit_patch(client: &reqwest::Client, base: &str, body: Value) -> Value {
    client
        .post(format!("{base}/v1/workspace/patches"))
        .json(&body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap()
}

async fn enqueue_patch(client: &reqwest::Client, base: &str, proposal_id: &str) -> Value {
    client
        .post(format!("{base}/v1/workspace/patches/{proposal_id}/enqueue"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap()
}

async fn process_next(client: &reqwest::Client, base: &str) -> reqwest::Response {
    client
        .post(format!("{base}/v1/workspace/merge/next"))
        .send()
        .await
        .unwrap()
}

async fn submit_enqueue(client: &reqwest::Client, base: &str, body: Value) -> Value {
    let proposal: Value = client
        .post(format!("{base}/v1/workspace/patches"))
        .json(&body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let proposal_id = proposal["id"].as_str().unwrap();
    enqueue_patch(client, base, proposal_id).await
}

#[tokio::test]
async fn merge_queue_merges_different_files_and_refuses_stale_same_file() {
    let (base, _dir) = boot().await;
    let client = reqwest::Client::new();
    for (agent, path) in [("agent-a", "src/lib.rs"), ("agent-b", "src/main.rs")] {
        let _: Value = client
            .post(format!("{base}/v1/workspace/leases"))
            .json(&claim(agent, path))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
    }

    let queued = submit_enqueue(&client, &base, patch("agent-a", "src/lib.rs", "next")).await;
    assert_eq!(queued["sequence"], 1);
    let queued = submit_enqueue(&client, &base, patch("agent-b", "src/main.rs", "main-next")).await;
    assert_eq!(queued["sequence"], 2);

    let queue: Value = client
        .get(format!("{base}/v1/workspace/merge-queue"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(queue[0]["status"], "pending");
    assert_eq!(queue[1]["status"], "pending");

    let resp = process_next(&client, &base).await;
    assert_eq!(resp.status(), 200);
    let merged: Value = resp.json().await.unwrap();
    assert_eq!(merged["item"]["sequence"], 1);
    assert_eq!(merged["item"]["status"], "merged");

    let resp = process_next(&client, &base).await;
    assert_eq!(resp.status(), 200);
    let merged: Value = resp.json().await.unwrap();
    assert_eq!(merged["item"]["sequence"], 2);
    assert_eq!(merged["item"]["status"], "merged");

    let stale = client
        .post(format!("{base}/v1/workspace/patches"))
        .json(&patch_hashes(
            "agent-a",
            "src/lib.rs",
            "old",
            "changed",
            "other",
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(stale.status(), 409);
    let body: Value = stale.json().await.unwrap();
    assert_eq!(body["error"]["code"], "workspace_patch_refused");

    let second = submit_patch(&client, &base, patch("agent-a", "src/lib.rs", "other")).await;
    let second_id = second["id"].as_str().unwrap();
    let queued = enqueue_patch(&client, &base, second_id).await;
    assert_eq!(queued["sequence"], 3);
    let resp = process_next(&client, &base).await;
    assert_eq!(resp.status(), 409);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"]["code"], "workspace_merge_refused");

    let conflicts: Value = client
        .get(format!("{base}/v1/workspace/conflicts"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let kinds = conflicts
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["kind"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(kinds.contains(&"stale_base"));
    assert!(kinds.contains(&"same_file_conflict"));

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
