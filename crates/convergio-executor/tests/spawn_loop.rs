//! Executor `spawn_loop` integration tests.

use chrono::Duration as ChronoDuration;
use convergio_db::Pool;
use convergio_durability::{init, Durability, PlanStatus, TaskStatus};
use convergio_executor::{spawn_loop, Executor, SpawnTemplate};
use convergio_lifecycle::Supervisor;
use convergio_planner::{Planner, PlannerMode};
use std::sync::Arc;
use std::time::Duration;
use tempfile::tempdir;

async fn fresh_with(template: SpawnTemplate) -> (Executor, Durability, tempfile::TempDir) {
    // Tests should not depend on operator env; these flags influence
    // dispatch semantics and can make assertions flaky.
    std::env::remove_var("CONVERGIO_EXECUTOR_USE_RUNNER");
    std::env::remove_var("CONVERGIO_EXECUTOR_MAX_PARALLEL");

    let dir = tempdir().unwrap();
    let url = format!("sqlite://{}/state.db", dir.path().display());
    let pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    convergio_lifecycle::init(&pool).await.unwrap();
    let dur = Durability::new(pool.clone());
    let sup = Supervisor::new(pool);
    let exec = Executor::new(dur.clone(), sup, template);
    (exec, dur, dir)
}

async fn fresh() -> (Executor, Durability, tempfile::TempDir) {
    fresh_with(SpawnTemplate::default()).await
}

#[tokio::test]
async fn spawn_loop_abort_stops_before_first_tick() {
    let (exec, dur, _dir) = fresh().await;
    let planner = Planner::new(dur.clone()).with_mode(PlannerMode::Heuristic);
    let plan_id = planner.solve("abort-task").await.unwrap();

    let handle = spawn_loop(Arc::new(exec), ChronoDuration::seconds(60));
    handle.abort();
    handle.abort();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let tasks = dur.tasks().list_by_plan(&plan_id).await.unwrap();
    assert!(tasks.iter().all(|t| t.status == TaskStatus::Pending));
}

#[tokio::test]
async fn spawn_loop_dispatches_pending_tasks_in_background() {
    // Wires the same loop the daemon's main.rs runs (ADR-0027). A
    // pending task with no wave dependencies must be promoted to
    // in_progress within one tick of the loop, with no manual
    // `Executor::tick()` or `POST /v1/dispatch` call.
    let (exec, dur, _dir) = fresh().await;
    let planner = Planner::new(dur.clone()).with_mode(PlannerMode::Heuristic);
    let plan_id = planner.solve("loop-task").await.unwrap();

    let handle = spawn_loop(Arc::new(exec), ChronoDuration::milliseconds(50));

    // Poll up to 5 seconds for the loop to flip the task. With a 50ms
    // tick and a single-task plan, the first round should land in
    // ~50-100ms; the budget is wide so this stays green on slow CI.
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let mut promoted = false;
    while std::time::Instant::now() < deadline {
        let tasks = dur.tasks().list_by_plan(&plan_id).await.unwrap();
        if tasks.iter().all(|t| t.status == TaskStatus::InProgress) {
            promoted = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    handle.abort();
    assert!(promoted, "spawn_loop did not dispatch within 5s");
}

#[tokio::test]
async fn spawn_loop_skips_cancelled_plans_and_dispatches_others() {
    // Regression: a cancelled plan has pending tasks that PlanStatusGate
    // will refuse. The loop must still dispatch other plans.
    let (exec, dur, _dir) = fresh().await;

    let blocked = dur
        .create_plan(convergio_durability::NewPlan {
            title: "blocked".into(),
            description: None,
            project: None,
        })
        .await
        .unwrap();
    let ok = dur
        .create_plan(convergio_durability::NewPlan {
            title: "ok".into(),
            description: None,
            project: None,
        })
        .await
        .unwrap();

    let blocked_task = dur
        .create_task(
            &blocked.id,
            convergio_durability::NewTask {
                wave: 1,
                sequence: 1,
                title: "blocked-task".into(),
                description: None,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();
    let ok_task = dur
        .create_task(
            &ok.id,
            convergio_durability::NewTask {
                wave: 1,
                sequence: 2,
                title: "ok-task".into(),
                description: None,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();

    dur.transition_plan(&blocked.id, PlanStatus::Cancelled)
        .await
        .unwrap();

    let handle = spawn_loop(Arc::new(exec), ChronoDuration::milliseconds(50));

    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let mut promoted = false;
    while std::time::Instant::now() < deadline {
        let ok_now = dur.tasks().get(&ok_task.id).await.unwrap();
        if ok_now.status == TaskStatus::InProgress {
            promoted = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    handle.abort();

    assert!(promoted, "spawn_loop did not dispatch ok plan within 5s");
    let blocked_now = dur.tasks().get(&blocked_task.id).await.unwrap();
    assert_eq!(blocked_now.status, TaskStatus::Pending);
}
