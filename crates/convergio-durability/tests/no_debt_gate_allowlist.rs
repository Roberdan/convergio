//! Tests for the F34 NoDebt allowlist.
//!
//! Split out of `no_debt_gate.rs` to honour the 300-line cap. These
//! cover the narrow exception that lets a doc/spec/adr evidence row
//! mention a debt keyword (TODO, WIP, FIXME, ...) when the task
//! title itself declares the task is *about* such an artifact.
//!
//! The two-half match (kind ∧ title) is on purpose: either half
//! alone would over-allow.

use convergio_db::Pool;
use convergio_durability::gates::{Gate, GateContext, NoDebtGate};
use convergio_durability::{init, Durability, NewPlan, NewTask, TaskStatus};
use serde_json::json;
use tempfile::tempdir;

async fn fresh() -> (Durability, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let url = format!("sqlite://{}/state.db", dir.path().display());
    let pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    (Durability::new(pool), dir)
}

fn ctx(dur: &Durability, task: convergio_durability::Task, target: TaskStatus) -> GateContext {
    GateContext {
        pool: dur.pool().clone(),
        task,
        target_status: target,
        agent_id: None,
    }
}

async fn make_titled_task(
    dur: &Durability,
    title: &str,
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
                title: title.into(),
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

#[tokio::test]
async fn allows_wip_in_doc_on_wip_titled_task() {
    let (dur, _dir) = fresh().await;
    let task = make_titled_task(
        &dur,
        "WIP commit message protocol",
        vec![(
            "doc",
            json!({"summary": "canonical WIP-commit pause-checklist"}),
        )],
    )
    .await;
    NoDebtGate::default()
        .check(&ctx(&dur, task, TaskStatus::Submitted))
        .await
        .expect("WIP in doc on WIP-titled task should be allowed (F34)");
}

#[tokio::test]
async fn still_refuses_unwrap_in_code_on_wip_titled_task() {
    let (dur, _dir) = fresh().await;
    let task = make_titled_task(
        &dur,
        "WIP commit message protocol",
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
async fn refuses_wip_in_doc_on_unrelated_task_title() {
    let (dur, _dir) = fresh().await;
    let task = make_titled_task(
        &dur,
        "Add user-facing dashboard",
        vec![("doc", json!({"summary": "WIP draft, leaving notes"}))],
    )
    .await;
    let err = NoDebtGate::default()
        .check(&ctx(&dur, task, TaskStatus::Submitted))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("todo_marker"));
}

#[tokio::test]
async fn allowlist_covers_kind_spec_too() {
    let (dur, _dir) = fresh().await;
    let task = make_titled_task(
        &dur,
        "Debt burn-down spec",
        vec![(
            "spec",
            json!({"text": "We track FIXME and TODO across the repo."}),
        )],
    )
    .await;
    NoDebtGate::default()
        .check(&ctx(&dur, task, TaskStatus::Submitted))
        .await
        .expect("FIXME/TODO mention in spec on debt-titled task should be allowed (F34)");
}
