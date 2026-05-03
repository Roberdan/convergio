//! Tests for `NoDebtGate`.
//!
//! These tests prove the leash works on the lamentela #1 ("agents
//! leave technical debt without telling you"). If they go red, an
//! agent could slip a TODO past the gate.

use convergio_db::Pool;
use convergio_durability::gates::{Gate, GateContext, NoDebtGate};
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

async fn make_task_with_evidence(
    dur: &Durability,
    evidence: Vec<(&str, serde_json::Value)>,
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
    for (kind, payload) in evidence {
        dur.attach_evidence(&task.id, kind, payload, Some(0))
            .await
            .unwrap();
    }
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
async fn passes_when_no_debt_markers() {
    let (dur, _dir) = fresh().await;
    let task = make_task_with_evidence(
        &dur,
        vec![(
            "code",
            json!({"diff": "fn add(a: i32, b: i32) -> i32 { a + b }"}),
        )],
    )
    .await;
    NoDebtGate::default()
        .check(&ctx(&dur, task, TaskStatus::Submitted))
        .await
        .unwrap();
}

#[tokio::test]
async fn refuses_todo_in_payload() {
    let (dur, _dir) = fresh().await;
    let task = make_task_with_evidence(
        &dur,
        vec![("code", json!({"diff": "// TODO: revisit\nfn x() {}"}))],
    )
    .await;
    let err = NoDebtGate::default()
        .check(&ctx(&dur, task, TaskStatus::Submitted))
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(matches!(
        err,
        DurabilityError::GateRefused {
            gate: "no_debt",
            ..
        }
    ));
    assert!(msg.contains("todo_marker"), "msg: {msg}");
}

#[tokio::test]
async fn refuses_unwrap_in_rust_code() {
    let (dur, _dir) = fresh().await;
    let task = make_task_with_evidence(
        &dur,
        vec![("code", json!({"diff": "let x = parse(s).unwrap();"}))],
    )
    .await;
    let err = NoDebtGate::default()
        .check(&ctx(&dur, task, TaskStatus::Submitted))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("rust_unwrap"));
}

#[tokio::test]
async fn refuses_ignored_test() {
    let (dur, _dir) = fresh().await;
    let task = make_task_with_evidence(
        &dur,
        vec![("code", json!({"diff": "#[ignore]\n#[test]\nfn flaky() {}"}))],
    )
    .await;
    let err = NoDebtGate::default()
        .check(&ctx(&dur, task, TaskStatus::Submitted))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("rust_ignored_test"));
}

#[tokio::test]
async fn refuses_console_log_in_js() {
    let (dur, _dir) = fresh().await;
    let task = make_task_with_evidence(
        &dur,
        vec![(
            "code",
            json!({"diff": "function go() { console.log('debug'); }"}),
        )],
    )
    .await;
    let err = NoDebtGate::default()
        .check(&ctx(&dur, task, TaskStatus::Submitted))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("js_console_log"));
}

#[tokio::test]
async fn no_op_for_in_progress_target() {
    // The gate only fires for Submitted/Done. Moving to InProgress
    // even with debt-laden evidence must pass.
    let (dur, _dir) = fresh().await;
    let task =
        make_task_with_evidence(&dur, vec![("code", json!({"diff": "// TODO\nfn x(){}"}))]).await;
    NoDebtGate::default()
        .check(&ctx(&dur, task, TaskStatus::InProgress))
        .await
        .unwrap();
}

#[tokio::test]
async fn lists_every_violation_kind() {
    // A single payload with multiple markers should report each.
    let (dur, _dir) = fresh().await;
    let task = make_task_with_evidence(
        &dur,
        vec![(
            "code",
            json!({"diff": "// TODO\nlet x = parse().unwrap();\n#[ignore]\n#[test] fn t(){}"}),
        )],
    )
    .await;
    let err = NoDebtGate::default()
        .check(&ctx(&dur, task, TaskStatus::Submitted))
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("todo_marker"), "msg: {msg}");
    assert!(msg.contains("rust_unwrap"), "msg: {msg}");
    assert!(msg.contains("rust_ignored_test"), "msg: {msg}");
}

#[tokio::test]
async fn fires_through_full_facade_pipeline() {
    // Integration: drive the gate through Durability::transition_task,
    // proving the pipeline wires it correctly.
    let (dur, _dir) = fresh().await;
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
                evidence_required: vec!["code".into()],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();
    dur.transition_task(&task.id, TaskStatus::InProgress, Some("a"))
        .await
        .unwrap();
    dur.attach_evidence(
        &task.id,
        "code",
        json!({"diff": "fn x() { todo!() }"}),
        Some(0),
    )
    .await
    .unwrap();

    let err = dur
        .transition_task(&task.id, TaskStatus::Submitted, Some("a"))
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("no_debt"), "msg: {msg}");
    assert!(msg.contains("rust_todo_macro"), "msg: {msg}");
}
