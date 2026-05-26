//! Quickstart E2E — proves the README's "60-second" claim.
//!
//! Pipeline:
//! 1. POST /v1/solve — turn a mission into a plan
//! 2. POST /v1/dispatch — executor moves wave 1 tasks to in_progress
//!    via Layer 3 spawn
//! 3. Force every task to done (in real life the agents do this; the
//!    test simulates it via direct HTTP calls)
//! 4. POST /v1/plans/:id/validate — Thor returns Pass

mod common;

use common::boot as common_boot;
use convergio_db::Pool;
use serde_json::{json, Value};

async fn boot() -> (String, Pool, tempfile::TempDir) {
    // Force the deterministic line-split planner so the E2E does
    // not invoke the operator's local `claude -p --model opus`
    // (ADR-0036) — that would charge real tokens on each run.
    std::env::set_var("CONVERGIO_PLANNER_MODE", "heuristic");

    // Tests must not depend on operator env. Clear tuning knobs that
    // change dispatch behavior or flip the executor into runner mode.
    std::env::remove_var("CONVERGIO_EXECUTOR_USE_RUNNER");
    std::env::remove_var("CONVERGIO_EXECUTOR_MAX_PARALLEL");

    common_boot().await
}

#[tokio::test]
async fn solve_dispatch_validate_full_loop() {
    let (base, pool, _dir) = boot().await;
    let c = common::client();

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
    let c = common::client();

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
