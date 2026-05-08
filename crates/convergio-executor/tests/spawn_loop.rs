//! Spawn-loop integration tests.

use chrono::Duration as ChronoDuration;
use convergio_db::Pool;
use convergio_durability::{init, Durability, TaskStatus};
use convergio_executor::{spawn_loop, Executor, SpawnTemplate};
use convergio_lifecycle::Supervisor;
use convergio_planner::{Planner, PlannerMode};
use std::sync::Arc;
use std::time::Duration;
use tempfile::tempdir;

fn clear_env() {
    for k in [
        "CONVERGIO_EXECUTOR_USE_RUNNER",
        "CONVERGIO_EXECUTOR_MAX_PARALLEL",
        "CONVERGIO_REPO_PATH",
    ] {
        std::env::remove_var(k);
    }
}

async fn fresh() -> (Executor, Durability, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let url = format!("sqlite://{}/state.db", dir.path().display());
    let pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    convergio_lifecycle::init(&pool).await.unwrap();
    let dur = Durability::new(pool.clone());
    let sup = Supervisor::new(pool);
    let exec = Executor::new(dur.clone(), sup, SpawnTemplate::default());
    (exec, dur, dir)
}

#[tokio::test]
async fn spawn_loop_abort_stops_before_first_tick() {
    clear_env();
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
    clear_env();
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
