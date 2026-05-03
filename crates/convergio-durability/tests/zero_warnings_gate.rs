//! Tests for `ZeroWarningsGate`.

use convergio_db::Pool;
use convergio_durability::gates::{Gate, GateContext, ZeroWarningsGate};
use convergio_durability::{init, Durability, DurabilityError, NewPlan, NewTask, TaskStatus};
use serde_json::json;
use tempfile::tempdir;

async fn fresh() -> (Durability, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let url = format!("sqlite://{}/state.db", dir.path().display());
    let pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    (Durability::new(pool), dir)
}

async fn task_with_evidence(
    dur: &Durability,
    kind: &str,
    payload: serde_json::Value,
    exit_code: Option<i64>,
) -> convergio_durability::Task {
    let plan = dur
        .create_plan(NewPlan {
            title: "p".into(),
            description: None,
            project: None,
        })
        .await
        .unwrap();
    let task = dur
        .create_task(
            &plan.id,
            NewTask {
                wave: 1,
                sequence: 1,
                title: "t".into(),
                description: None,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();
    dur.attach_evidence(&task.id, kind, payload, exit_code)
        .await
        .unwrap();
    dur.tasks().get(&task.id).await.unwrap()
}

fn ctx(dur: &Durability, task: convergio_durability::Task) -> GateContext {
    GateContext {
        pool: dur.pool().clone(),
        task,
        target_status: TaskStatus::Submitted,
        agent_id: None,
    }
}

#[tokio::test]
async fn passes_on_clean_build_evidence() {
    let (dur, _dir) = fresh().await;
    let task = task_with_evidence(
        &dur,
        "build",
        json!({"warnings_count": 0, "errors_count": 0}),
        Some(0),
    )
    .await;
    ZeroWarningsGate.check(&ctx(&dur, task)).await.unwrap();
}

#[tokio::test]
async fn refuses_on_nonzero_warnings() {
    let (dur, _dir) = fresh().await;
    let task = task_with_evidence(&dur, "lint", json!({"warnings_count": 3}), Some(0)).await;
    let err = ZeroWarningsGate.check(&ctx(&dur, task)).await.unwrap_err();
    let msg = err.to_string();
    assert!(matches!(
        err,
        DurabilityError::GateRefused {
            gate: "zero_warnings",
            ..
        }
    ));
    assert!(msg.contains("warnings"), "msg: {msg}");
}

#[tokio::test]
async fn refuses_on_nonzero_errors() {
    let (dur, _dir) = fresh().await;
    let task = task_with_evidence(&dur, "compile", json!({"errors_count": 1}), Some(0)).await;
    let err = ZeroWarningsGate.check(&ctx(&dur, task)).await.unwrap_err();
    assert!(err.to_string().contains("errors"));
}

#[tokio::test]
async fn refuses_on_nonzero_exit_code() {
    let (dur, _dir) = fresh().await;
    let task = task_with_evidence(&dur, "test", json!({}), Some(1)).await;
    let err = ZeroWarningsGate.check(&ctx(&dur, task)).await.unwrap_err();
    assert!(err.to_string().contains("nonzero_exit"));
}

#[tokio::test]
async fn refuses_on_failures_array() {
    let (dur, _dir) = fresh().await;
    let task = task_with_evidence(
        &dur,
        "test",
        json!({"failures": ["test_a", "test_b"]}),
        Some(0),
    )
    .await;
    let err = ZeroWarningsGate.check(&ctx(&dur, task)).await.unwrap_err();
    assert!(err.to_string().contains("failures"));
}

#[tokio::test]
async fn ignores_unrelated_evidence_kinds() {
    // A `manual` evidence row isn't a quality signal — even if it has
    // a "warnings_count" field it must NOT trigger the gate.
    let (dur, _dir) = fresh().await;
    let task = task_with_evidence(
        &dur,
        "manual",
        json!({"warnings_count": 99, "comment": "this is not a build"}),
        Some(0),
    )
    .await;
    ZeroWarningsGate.check(&ctx(&dur, task)).await.unwrap();
}

#[tokio::test]
async fn no_op_for_in_progress_target() {
    let (dur, _dir) = fresh().await;
    let task = task_with_evidence(&dur, "lint", json!({"warnings_count": 5}), Some(0)).await;
    let ctx_in_progress = GateContext {
        pool: dur.pool().clone(),
        task,
        target_status: TaskStatus::InProgress,
        agent_id: None,
    };
    ZeroWarningsGate.check(&ctx_in_progress).await.unwrap();
}

#[tokio::test]
async fn covers_us_singular_warning_count_field() {
    let (dur, _dir) = fresh().await;
    let task = task_with_evidence(&dur, "lint", json!({"warning_count": 7}), Some(0)).await;
    let err = ZeroWarningsGate.check(&ctx(&dur, task)).await.unwrap_err();
    assert!(err.to_string().contains("warnings"));
}
