//! Tests for `NoSecretsGate`.

use convergio_db::Pool;
use convergio_durability::gates::{Gate, GateContext, NoSecretsGate};
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

async fn make_task(dur: &Durability, payload: serde_json::Value) -> convergio_durability::Task {
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
    dur.attach_evidence(&task.id, "code", payload, Some(0))
        .await
        .unwrap();
    dur.tasks().get(&task.id).await.unwrap()
}

fn ctx(dur: &Durability, task: convergio_durability::Task, target: TaskStatus) -> GateContext {
    GateContext {
        pool: dur.pool().clone(),
        task,
        target_status: target,
        agent_id: None,
    }
}

#[tokio::test]
async fn passes_when_no_secret_markers() {
    let (dur, _dir) = fresh().await;
    let task = make_task(&dur, json!({"diff": "fn handler() -> bool { true }"})).await;
    NoSecretsGate::default()
        .check(&ctx(&dur, task, TaskStatus::Submitted))
        .await
        .unwrap();
}

#[tokio::test]
async fn refuses_private_key() {
    let (dur, _dir) = fresh().await;
    let task = make_task(
        &dur,
        json!({"diff": "-----BEGIN PRIVATE KEY-----\nabc\n-----END PRIVATE KEY-----"}),
    )
    .await;
    let err = NoSecretsGate::default()
        .check(&ctx(&dur, task, TaskStatus::Submitted))
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        DurabilityError::GateRefused {
            gate: "no_secrets",
            ..
        }
    ));
    assert!(err.to_string().contains("private_key"));
}

#[tokio::test]
async fn refuses_common_service_tokens() {
    let (dur, _dir) = fresh().await;
    let task = make_task(
        &dur,
        json!({"diff": "let token = \"ghp_abcdefghijklmnopqrstuvwxyzABCDEFGHIJ\";"}),
    )
    .await;
    let err = NoSecretsGate::default()
        .check(&ctx(&dur, task, TaskStatus::Done))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("github_token"));
}

#[tokio::test]
async fn no_op_for_in_progress_target() {
    let (dur, _dir) = fresh().await;
    let task = make_task(
        &dur,
        json!({"diff": "-----BEGIN PRIVATE KEY-----\nabc\n-----END PRIVATE KEY-----"}),
    )
    .await;
    NoSecretsGate::default()
        .check(&ctx(&dur, task, TaskStatus::InProgress))
        .await
        .unwrap();
}
