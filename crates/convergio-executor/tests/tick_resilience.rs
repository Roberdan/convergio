//! Tick resilience integration tests.

use convergio_db::Pool;
use convergio_durability::{init, Durability, TaskStatus};
use convergio_executor::{Executor, SpawnTemplate};
use convergio_lifecycle::Supervisor;
use tempfile::tempdir;

async fn fresh() -> (Executor, Durability, tempfile::TempDir) {
    std::env::remove_var("CONVERGIO_EXECUTOR_USE_RUNNER");
    std::env::remove_var("CONVERGIO_EXECUTOR_MAX_PARALLEL");

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
async fn tick_skips_failing_task_and_keeps_dispatching_with_budget() {
    let (exec, dur, _dir) = fresh().await;
    std::env::set_var("CONVERGIO_EXECUTOR_MAX_PARALLEL", "1");
    let plan = dur
        .create_plan(convergio_durability::NewPlan {
            title: "p".into(),
            description: None,
            project: None,
        })
        .await
        .unwrap();

    let t1 = dur
        .create_task(
            &plan.id,
            convergio_durability::NewTask {
                wave: 1,
                sequence: 1,
                title: "runner-fails".into(),
                description: None,
                evidence_required: vec![],
                runner_kind: Some("copilot:gpt-5.2".into()),
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();

    let t2 = dur
        .create_task(
            &plan.id,
            convergio_durability::NewTask {
                wave: 1,
                sequence: 2,
                title: "legacy-succeeds".into(),
                description: None,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();

    let n = exec.tick().await.unwrap();
    assert_eq!(n, 1);

    let after1 = dur.tasks().get(&t1.id).await.unwrap();
    let after2 = dur.tasks().get(&t2.id).await.unwrap();
    assert_eq!(after1.status, TaskStatus::Pending);
    assert_eq!(after2.status, TaskStatus::InProgress);
    assert!(after2.agent_id.is_some());
    assert!(dur.audit().verify(None, None).await.unwrap().ok);

    std::env::remove_var("CONVERGIO_EXECUTOR_MAX_PARALLEL");
}
