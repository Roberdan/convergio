//! Executor integration tests.

use convergio_db::Pool;
use convergio_durability::{init, Durability, TaskStatus};
use convergio_executor::{Executor, SpawnTemplate};
use convergio_lifecycle::Supervisor;
use convergio_planner::{Planner, PlannerMode};
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

async fn fresh_with(template: SpawnTemplate) -> (Executor, Durability, tempfile::TempDir) {
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
async fn tick_dispatches_pending_tasks_in_first_wave() {
    clear_env();
    let (exec, dur, _dir) = fresh().await;
    let planner = Planner::new(dur.clone()).with_mode(PlannerMode::Heuristic);
    let plan_id = planner.solve("a\nb\nc").await.unwrap();

    let dispatched = exec.tick().await.unwrap();
    assert_eq!(dispatched, 3);

    let tasks = dur.tasks().list_by_plan(&plan_id).await.unwrap();
    assert!(tasks.iter().all(|t| t.status == TaskStatus::InProgress));
    assert!(tasks.iter().all(|t| t.agent_id.is_some()));
}

#[tokio::test]
async fn tick_skips_later_waves_until_earlier_done() {
    clear_env();
    let (exec, dur, _dir) = fresh().await;
    let plan = dur
        .create_plan(convergio_durability::NewPlan {
            title: "p".into(),
            description: None,
            project: None,
        })
        .await
        .unwrap();
    let _w1 = dur
        .create_task(
            &plan.id,
            convergio_durability::NewTask {
                wave: 1,
                sequence: 1,
                title: "wave1".into(),
                description: None,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();
    let w2 = dur
        .create_task(
            &plan.id,
            convergio_durability::NewTask {
                wave: 2,
                sequence: 1,
                title: "wave2".into(),
                description: None,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();

    // First tick: only wave 1 dispatched.
    let n = exec.tick().await.unwrap();
    assert_eq!(n, 1);
    let after = dur.tasks().get(&w2.id).await.unwrap();
    assert_eq!(after.status, TaskStatus::Pending);

    // Second tick: wave 1 still in_progress so wave 2 still waits.
    let n = exec.tick().await.unwrap();
    assert_eq!(n, 0);
}

#[tokio::test]
async fn tick_dispatches_later_wave_after_earlier_failed() {
    clear_env();
    let (exec, dur, _dir) = fresh().await;
    let plan = dur
        .create_plan(convergio_durability::NewPlan {
            title: "p".into(),
            description: None,
            project: None,
        })
        .await
        .unwrap();
    let w1 = dur
        .create_task(
            &plan.id,
            convergio_durability::NewTask {
                wave: 1,
                sequence: 1,
                title: "wave1".into(),
                description: None,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();
    let w2 = dur
        .create_task(
            &plan.id,
            convergio_durability::NewTask {
                wave: 2,
                sequence: 1,
                title: "wave2".into(),
                description: None,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();
    dur.transition_task(&w1.id, TaskStatus::InProgress, Some("agent-1"))
        .await
        .unwrap();
    dur.transition_task(&w1.id, TaskStatus::Failed, Some("agent-1"))
        .await
        .unwrap();

    let n = exec.tick().await.unwrap();
    assert_eq!(n, 1);
    let after = dur.tasks().get(&w2.id).await.unwrap();
    assert_eq!(after.status, TaskStatus::InProgress);
}

#[tokio::test]
async fn tick_does_not_steal_already_claimed_task() {
    clear_env();
    let (exec, dur, _dir) = fresh().await;
    let planner = Planner::new(dur.clone()).with_mode(PlannerMode::Heuristic);
    let plan_id = planner.solve("claimed").await.unwrap();
    let task = dur.tasks().list_by_plan(&plan_id).await.unwrap().remove(0);
    dur.transition_task(&task.id, TaskStatus::InProgress, Some("manual-agent"))
        .await
        .unwrap();

    let n = exec.tick().await.unwrap();
    let after = dur.tasks().get(&task.id).await.unwrap();
    assert_eq!(n, 0);
    assert_eq!(after.status, TaskStatus::InProgress);
    assert_eq!(after.agent_id.as_deref(), Some("manual-agent"));
}

#[tokio::test]
async fn tick_is_idempotent_on_already_dispatched_tasks() {
    clear_env();
    let (exec, dur, _dir) = fresh().await;
    let planner = Planner::new(dur.clone()).with_mode(PlannerMode::Heuristic);
    planner.solve("only one").await.unwrap();

    let n1 = exec.tick().await.unwrap();
    let n2 = exec.tick().await.unwrap();
    assert_eq!(n1, 1);
    assert_eq!(n2, 0);
}

#[tokio::test]
async fn tick_leaves_task_pending_when_spawn_fails() {
    clear_env();
    let (exec, dur, _dir) = fresh_with(SpawnTemplate {
        command: "/definitely-not-convergio-executor-test".into(),
        args: vec![],
        kind: "missing".into(),
    })
    .await;
    let planner = Planner::new(dur.clone()).with_mode(PlannerMode::Heuristic);
    let plan_id = planner.solve("spawn-failure").await.unwrap();
    let task = dur.tasks().list_by_plan(&plan_id).await.unwrap().remove(0);

    // spawn failure is logged and swallowed — tick() returns Ok(0)
    let n = exec.tick().await.unwrap();
    assert_eq!(n, 0);
    let after = dur.tasks().get(&task.id).await.unwrap();
    assert_eq!(after.status, TaskStatus::Pending);
    assert!(after.agent_id.is_none());
    assert!(dur.audit().verify(None, None).await.unwrap().ok);
}

#[tokio::test]
async fn tick_attempts_all_tasks_even_if_first_spawn_fails() {
    clear_env();
    // Regression: tick() used `?` on dispatch_one(), aborting the whole
    // batch on the first failure. Now it logs and continues.
    let (exec, dur, _dir) = fresh_with(SpawnTemplate {
        command: "/definitely-not-convergio-executor-test".into(),
        args: vec![],
        kind: "missing".into(),
    })
    .await;
    let planner = Planner::new(dur.clone()).with_mode(PlannerMode::Heuristic);
    let plan_id = planner.solve("a\nb\nc").await.unwrap();

    // All 3 spawns will fail; tick() should return Ok(0) not Err(_).
    let n = exec.tick().await.unwrap();
    assert_eq!(n, 0);

    // All 3 tasks remain pending — none were silently left unprocessed.
    let tasks = dur.tasks().list_by_plan(&plan_id).await.unwrap();
    assert_eq!(tasks.len(), 3);
    assert!(tasks.iter().all(|t| t.status == TaskStatus::Pending));
    assert!(dur.audit().verify(None, None).await.unwrap().ok);
}

#[tokio::test]
async fn dispatch_writes_audit_chain_that_verifies() {
    clear_env();
    let (exec, dur, _dir) = fresh().await;
    let planner = Planner::new(dur.clone()).with_mode(PlannerMode::Heuristic);
    planner.solve("x\ny").await.unwrap();
    exec.tick().await.unwrap();

    let r = dur.audit().verify(None, None).await.unwrap();
    assert!(r.ok, "{r:?}");
    // 1 plan.created + 2 task.created + 2 task.in_progress = 5+
    assert!(r.checked >= 5);
}
