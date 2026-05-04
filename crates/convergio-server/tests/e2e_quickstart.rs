//! Quickstart E2E — proves the README's "60-second" claim.
//!
//! Pipeline:
//! 1. POST /v1/solve — turn a mission into a plan
//! 2. POST /v1/dispatch — executor moves wave 1 tasks to in_progress
//!    via Layer 3 spawn
//! 3. Force every task to done (in real life the agents do this; the
//!    test simulates it via direct HTTP calls)
//! 4. POST /v1/plans/:id/validate — Thor returns Pass

use convergio_bus::Bus;
use convergio_db::Pool;
use convergio_durability::{init, Durability};
use convergio_lifecycle::Supervisor;
use convergio_server::{router, AppState};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::net::TcpListener;

async fn boot() -> (String, Pool, tempfile::TempDir) {
    // Force the deterministic line-split planner so the E2E does
    // not invoke the operator's local `claude -p --model opus`
    // (ADR-0036) — that would charge real tokens on each run.
    std::env::set_var("CONVERGIO_PLANNER_MODE", "heuristic");
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("state.db");
    let url = format!("sqlite://{}", db_path.display());
    let pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    convergio_bus::init(&pool).await.unwrap();
    convergio_lifecycle::init(&pool).await.unwrap();

    let state = AppState {
        durability: Arc::new(Durability::new(pool.clone())),
        bus: Arc::new(Bus::new(pool.clone())),
        supervisor: Arc::new(Supervisor::new(pool.clone())),
        graph: Arc::new(convergio_graph::Store::new(pool.clone())),
        embed: Arc::new(convergio_embed::EmbedStore::new(pool.clone())),
        embedder: Arc::new(convergio_embed::embedder::testing::DeterministicTestEmbedder::new(8)),
        fleet: Arc::new(convergio_fleet::FleetStore::new(pool.clone())),
    };
    let app = router(state);

    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{addr}"), pool, dir)
}

#[tokio::test]
async fn solve_dispatch_validate_full_loop() {
    let (base, pool, _dir) = boot().await;
    let c = reqwest::Client::new();

    // 1. Solve a mission.
    let solved: Value = c
        .post(format!("{base}/v1/solve"))
        .json(&json!({"mission": "ship convergio v3\nwrite the demo\nopen-source it"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let plan_id = solved["plan_id"].as_str().unwrap().to_string();

    // The plan now has 3 tasks in wave 1.
    let tasks: Vec<Value> = c
        .get(format!("{base}/v1/plans/{plan_id}/tasks"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(tasks.len(), 3);

    // 2. Dispatch — executor moves them to in_progress and spawns
    //    /bin/echo for each.
    let dispatch: Value = c
        .post(format!("{base}/v1/dispatch"))
        .json(&json!({}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(dispatch["dispatched"], 3);

    // 3. Force every task to done. (Real agents would attach evidence
    //    + transition; the executor's job stops at dispatch.)
    for t in &tasks {
        let task_id = t["id"].as_str().unwrap();
        // Skip submitted; go straight from in_progress to done via
        // direct DB write (the gate pipeline allows it; submitted is
        // just an interstitial). We use the same pool to avoid HTTP
        // ceremony.
        sqlx::query("UPDATE tasks SET status = 'done' WHERE id = ?")
            .bind(task_id)
            .execute(pool.inner())
            .await
            .unwrap();
    }

    // 4. Validate — Thor returns Pass.
    let verdict: Value = c
        .post(format!("{base}/v1/plans/{plan_id}/validate"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(verdict["verdict"], "pass", "verdict: {verdict}");

    // 5. Sanity: the audit chain still verifies.
    let report: Value = c
        .get(format!("{base}/v1/audit/verify"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(report["ok"], true);
}

#[tokio::test]
async fn validate_returns_fail_on_open_tasks() {
    let (base, _pool, _dir) = boot().await;
    let c = reqwest::Client::new();

    let solved: Value = c
        .post(format!("{base}/v1/solve"))
        .json(&json!({"mission": "alpha\nbeta"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let plan_id = solved["plan_id"].as_str().unwrap();

    let verdict: Value = c
        .post(format!("{base}/v1/plans/{plan_id}/validate"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(verdict["verdict"], "fail");
    assert!(
        verdict["reasons"].as_array().unwrap().len() >= 2,
        "verdict: {verdict}"
    );
}
