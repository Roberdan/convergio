//! E2E for the materialised timing cache (ADR-0031).
//!
//! Drives a task through `pending → in_progress → submitted → done`
//! over HTTP and verifies that the `started_at`, `ended_at`, and
//! `duration_ms` columns on the task row track the transition
//! timestamps, all in the same audit-chained transaction.

use convergio_bus::Bus;
use convergio_db::Pool;
use convergio_durability::{init, Durability, NewTask};
use convergio_lifecycle::Supervisor;
use convergio_server::{router, AppState};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::net::TcpListener;

async fn boot() -> (String, AppState, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("state.db");
    let url = format!("sqlite://{}", db_path.display());
    let pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    convergio_bus::init(&pool).await.unwrap();
    convergio_lifecycle::init(&pool).await.unwrap();

    let durability = Arc::new(Durability::new(pool.clone()));
    let state = AppState {
        durability: durability.clone(),
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
    let app = router(state.clone());
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{addr}"), state, dir)
}

#[tokio::test]
async fn task_timing_cache_tracks_in_progress_then_done() {
    let (base, state, _dir) = boot().await;
    let client = reqwest::Client::new();

    let plan: Value = client
        .post(format!("{base}/v1/plans"))
        .json(&json!({ "title": "timing-cache" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let plan_id = plan.get("id").and_then(Value::as_str).unwrap().to_string();

    let task = state
        .durability
        .create_task(
            &plan_id,
            NewTask {
                wave: 1,
                sequence: 1,
                title: "timed".into(),
                description: None,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();

    // Pending → in_progress should set started_at and leave ended_at NULL.
    state
        .durability
        .transition_task(&task.id, convergio_durability::TaskStatus::InProgress, None)
        .await
        .unwrap();
    let after_start = state.durability.tasks().get(&task.id).await.unwrap();
    assert!(after_start.started_at.is_some(), "started_at must be set");
    assert!(
        after_start.ended_at.is_none(),
        "ended_at must still be NULL"
    );
    assert!(after_start.duration_ms.is_none());

    // In-progress → submitted leaves the cache columns alone.
    state
        .durability
        .transition_task(&task.id, convergio_durability::TaskStatus::Submitted, None)
        .await
        .unwrap();
    let after_submit = state.durability.tasks().get(&task.id).await.unwrap();
    assert_eq!(after_submit.started_at, after_start.started_at);
    assert!(after_submit.ended_at.is_none());

    // Sleep ~50ms so duration_ms is non-zero.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Submitted → done (via Thor's complete_validated_tasks) sets
    // ended_at + duration_ms.
    state
        .durability
        .complete_validated_tasks(std::slice::from_ref(&task.id))
        .await
        .unwrap();
    let after_done = state.durability.tasks().get(&task.id).await.unwrap();
    assert_eq!(after_done.started_at, after_start.started_at);
    assert!(
        after_done.ended_at.is_some(),
        "ended_at must be set on done"
    );
    assert!(
        after_done.duration_ms.unwrap_or(0) >= 50,
        "duration_ms must be >= sleep window: {:?}",
        after_done.duration_ms
    );

    // Audit chain integrity check.
    let verify: Value = client
        .get(format!("{base}/v1/audit/verify"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(verify.get("ok").and_then(Value::as_bool), Some(true));
}

#[tokio::test]
async fn plan_timing_cache_tracks_active_then_completed() {
    let (base, state, _dir) = boot().await;
    let client = reqwest::Client::new();

    let plan: Value = client
        .post(format!("{base}/v1/plans"))
        .json(&json!({ "title": "plan-timing" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let plan_id = plan.get("id").and_then(Value::as_str).unwrap().to_string();

    let _ = client
        .post(format!("{base}/v1/plans/{plan_id}/transition"))
        .json(&json!({ "target": "active" }))
        .send()
        .await
        .unwrap();
    let after_active = state.durability.plans().get(&plan_id).await.unwrap();
    assert!(after_active.started_at.is_some());
    assert!(after_active.ended_at.is_none());

    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

    let _ = client
        .post(format!("{base}/v1/plans/{plan_id}/transition"))
        .json(&json!({ "target": "completed" }))
        .send()
        .await
        .unwrap();
    let after_done = state.durability.plans().get(&plan_id).await.unwrap();
    assert!(after_done.ended_at.is_some());
    assert!(after_done.duration_ms.unwrap_or(0) >= 20);
}

#[tokio::test]
async fn close_post_hoc_writes_timing_cache() {
    // ADR-0031: the second public path to `done` (operator post-hoc
    // close) must also write `ended_at` and `duration_ms`, otherwise
    // tasks closed this way show up with stale/null timing in the
    // dashboard. Caught in PR #118 review.
    let (_base, state, _dir) = boot().await;

    let plan = state
        .durability
        .create_plan(convergio_durability::NewPlan {
            title: "post-hoc-timing".into(),
            description: None,
            project: None,
        })
        .await
        .unwrap();
    let task = state
        .durability
        .create_task(
            &plan.id,
            NewTask {
                wave: 1,
                sequence: 1,
                title: "phx".into(),
                description: None,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();

    state
        .durability
        .transition_task(&task.id, convergio_durability::TaskStatus::InProgress, None)
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;

    state
        .durability
        .close_task_post_hoc(&task.id, "merged in PR #999 outside the daemon", None)
        .await
        .unwrap();

    let after = state.durability.tasks().get(&task.id).await.unwrap();
    assert!(after.started_at.is_some());
    assert!(after.ended_at.is_some(), "post-hoc close must set ended_at");
    assert!(
        after.duration_ms.unwrap_or(0) >= 30,
        "post-hoc close must compute duration_ms: {:?}",
        after.duration_ms
    );
}

#[tokio::test]
async fn audit_verify_cache_is_populated_and_invalidated_on_append() {
    let (base, state, _dir) = boot().await;
    let client = reqwest::Client::new();

    client
        .post(format!("{base}/v1/plans"))
        .json(&json!({ "title": "cache-seed" }))
        .send()
        .await
        .unwrap();

    // First call: cache miss — full chain verify runs.
    let r1: serde_json::Value = client
        .get(format!("{base}/v1/audit/verify"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(r1["ok"], true);

    let cached_seq = {
        let g = state.audit_verify_cache.lock().unwrap();
        g.as_ref()
            .expect("cache must be populated after first verify")
            .0
    };

    // New append → tail advances → next call must be a cache miss.
    client
        .post(format!("{base}/v1/plans"))
        .json(&json!({ "title": "cache-bust" }))
        .send()
        .await
        .unwrap();

    let r3: serde_json::Value = client
        .get(format!("{base}/v1/audit/verify"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(r3["ok"], true);

    let new_seq = state.audit_verify_cache.lock().unwrap().as_ref().unwrap().0;
    assert!(
        new_seq > cached_seq,
        "cache seq must advance after new append"
    );
}
