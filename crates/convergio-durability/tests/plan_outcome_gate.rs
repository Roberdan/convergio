//! Tests for `PlanOutcomeGate` (W4 / ADR-0055).

use convergio_db::Pool;
use convergio_durability::gates::{Gate, GateContext, PlanOutcomeGate};
use convergio_durability::{init, Durability, DurabilityError, NewPlan, NewTask, TaskStatus};
use tempfile::TempDir;

async fn fresh_pool() -> (Pool, TempDir) {
    let dir = TempDir::new().expect("tmp");
    let url = format!("sqlite://{}/state.db", dir.path().display());
    let pool: Pool = Pool::connect(&url).await.expect("pool");
    init(&pool).await.expect("migrate");
    (pool, dir)
}

fn ctx(pool: Pool, task: convergio_durability::Task) -> GateContext {
    GateContext {
        pool,
        task,
        target_status: TaskStatus::Done,
        agent_id: None,
    }
}

async fn create_plan(dur: &Durability) -> String {
    dur.create_plan(NewPlan {
        title: "plan".into(),
        description: None,
        project: None,
    })
    .await
    .unwrap()
    .id
}

async fn create_task(dur: &Durability, plan_id: &str, seq: i64) -> convergio_durability::Task {
    dur.create_task(
        plan_id,
        NewTask {
            wave: 1,
            sequence: seq,
            title: format!("task-{seq}"),
            description: None,
            evidence_required: vec![],
            runner_kind: None,
            profile: None,
            max_budget_usd: None,
        },
    )
    .await
    .unwrap()
}

async fn set_status(pool: &Pool, task_id: &str, status: &str) {
    sqlx::query("UPDATE tasks SET status = ? WHERE id = ?")
        .bind(status)
        .bind(task_id)
        .execute(pool.inner())
        .await
        .unwrap();
}

#[tokio::test]
async fn no_op_when_env_unset() {
    std::env::remove_var("CONVERGIO_REQUIRE_PLAN_OUTCOME");
    let (pool, _dir) = fresh_pool().await;
    let dur = Durability::new(pool.clone());
    let plan_id = create_plan(&dur).await;
    let task = create_task(&dur, &plan_id, 1).await;
    PlanOutcomeGate::new()
        .check(&ctx(pool, task))
        .await
        .unwrap();
}

#[tokio::test]
async fn non_done_target_is_no_op() {
    std::env::set_var("CONVERGIO_REQUIRE_PLAN_OUTCOME", "1");
    let (pool, _dir) = fresh_pool().await;
    let dur = Durability::new(pool.clone());
    let plan_id = create_plan(&dur).await;
    let task = create_task(&dur, &plan_id, 1).await;
    let c = GateContext {
        pool,
        task,
        target_status: TaskStatus::Submitted,
        agent_id: None,
    };
    PlanOutcomeGate::new().check(&c).await.unwrap();
}

#[tokio::test]
async fn passes_for_intermediate_task_done() {
    std::env::set_var("CONVERGIO_REQUIRE_PLAN_OUTCOME", "1");
    let (pool, _dir) = fresh_pool().await;
    let dur = Durability::new(pool.clone());
    let plan_id = create_plan(&dur).await;
    let t1 = create_task(&dur, &plan_id, 1).await;
    let _t2 = create_task(&dur, &plan_id, 2).await;
    // t2 still pending — plan not closing yet.
    PlanOutcomeGate::new().check(&ctx(pool, t1)).await.unwrap();
}

#[tokio::test]
async fn refuses_when_success_rate_too_low() {
    std::env::set_var("CONVERGIO_REQUIRE_PLAN_OUTCOME", "1");
    let (pool, _dir) = fresh_pool().await;
    let dur = Durability::new(pool.clone());
    let plan_id = create_plan(&dur).await;
    // 5 tasks: 3 failed, 1 done, 1 closing → 2/5 = 40 % < 80 %
    let t1 = create_task(&dur, &plan_id, 1).await;
    let t2 = create_task(&dur, &plan_id, 2).await;
    let t3 = create_task(&dur, &plan_id, 3).await;
    let t4 = create_task(&dur, &plan_id, 4).await;
    let t5 = create_task(&dur, &plan_id, 5).await;
    set_status(&pool, &t1.id, "failed").await;
    set_status(&pool, &t2.id, "failed").await;
    set_status(&pool, &t3.id, "failed").await;
    set_status(&pool, &t4.id, "done").await;
    let t5_fresh = dur.tasks().get(&t5.id).await.unwrap();
    let err = PlanOutcomeGate::new()
        .check(&ctx(pool, t5_fresh))
        .await
        .unwrap_err();
    match err {
        DurabilityError::GateRefused { gate, reason } => {
            assert_eq!(gate, "plan_outcome");
            assert!(
                reason.contains("plan_success_rate_too_low"),
                "unexpected reason: {reason}"
            );
        }
        other => panic!("expected GateRefused, got {other:?}"),
    }
}

#[tokio::test]
async fn passes_when_success_rate_at_threshold() {
    std::env::set_var("CONVERGIO_REQUIRE_PLAN_OUTCOME", "1");
    let (pool, _dir) = fresh_pool().await;
    let dur = Durability::new(pool.clone());
    let plan_id = create_plan(&dur).await;
    // 5 tasks: 1 failed, 4 done → 4/5 = 80 % = threshold
    let t1 = create_task(&dur, &plan_id, 1).await;
    let t2 = create_task(&dur, &plan_id, 2).await;
    let t3 = create_task(&dur, &plan_id, 3).await;
    let t4 = create_task(&dur, &plan_id, 4).await;
    let t5 = create_task(&dur, &plan_id, 5).await;
    set_status(&pool, &t1.id, "failed").await;
    set_status(&pool, &t2.id, "done").await;
    set_status(&pool, &t3.id, "done").await;
    set_status(&pool, &t4.id, "done").await;
    let t5_fresh = dur.tasks().get(&t5.id).await.unwrap();
    PlanOutcomeGate::new()
        .check(&ctx(pool, t5_fresh))
        .await
        .unwrap();
}

#[tokio::test]
async fn passes_with_custom_lower_threshold() {
    std::env::set_var("CONVERGIO_REQUIRE_PLAN_OUTCOME", "1");
    let (pool, _dir) = fresh_pool().await;
    let dur = Durability::new(pool.clone());
    let plan_id = create_plan(&dur).await;
    // 2 tasks: 1 failed, 1 closing → 1/2 = 50 % at threshold 50 %
    let t1 = create_task(&dur, &plan_id, 1).await;
    let t2 = create_task(&dur, &plan_id, 2).await;
    set_status(&pool, &t1.id, "failed").await;
    let t2_fresh = dur.tasks().get(&t2.id).await.unwrap();
    PlanOutcomeGate::with_threshold(0.5)
        .check(&ctx(pool, t2_fresh))
        .await
        .unwrap();
}
