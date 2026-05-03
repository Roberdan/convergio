//! Tests for `NoStubGate` (P4 — no scaffolding only).
//!

use convergio_db::Pool;
use convergio_durability::gates::{Gate, GateContext, NoStubGate};
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

async fn task_with_diff(dur: &Durability, diff: &str) -> convergio_durability::Task {
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
    dur.attach_evidence(&task.id, "code", json!({"diff": diff}), Some(0))
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

async fn refuse(dur: &Durability, diff: &str, expected_rule: &str) {
    let task = task_with_diff(dur, diff).await;
    let err = NoStubGate::default()
        .check(&ctx(dur, task))
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains(expected_rule),
        "expected `{expected_rule}` in error for diff `{diff}`: {msg}"
    );
}

async fn pass(dur: &Durability, diff: &str) {
    let task = task_with_diff(dur, diff).await;
    NoStubGate::default()
        .check(&ctx(dur, task))
        .await
        .unwrap_or_else(|e| panic!("expected pass for `{diff}`, got: {e}"));
}

#[tokio::test]
async fn stub_comment_in_rust_double_slash() {
    let (dur, _dir) = fresh().await;
    refuse(&dur, "// stub\nfn x() {}", "stub_comment").await;
}

#[tokio::test]
async fn stub_comment_in_python_hash() {
    let (dur, _dir) = fresh().await;
    refuse(
        &dur,
        "# stub: implement later\ndef x():\n    pass",
        "stub_comment",
    )
    .await;
}

#[tokio::test]
async fn stub_comment_in_sql_double_dash() {
    let (dur, _dir) = fresh().await;
    refuse(
        &dur,
        "-- stub\nCREATE TABLE x (id INTEGER);",
        "stub_comment",
    )
    .await;
}

#[tokio::test]
async fn stub_comment_in_html() {
    let (dur, _dir) = fresh().await;
    refuse(
        &dur,
        "<!-- placeholder -->\n<div></div>",
        "placeholder_comment",
    )
    .await;
}

#[tokio::test]
async fn stub_comment_in_css_block() {
    let (dur, _dir) = fresh().await;
    refuse(
        &dur,
        "/* scaffolding */\n.btn { color: red; }",
        "scaffold_comment",
    )
    .await;
}

#[tokio::test]
async fn stub_comment_in_bash_hash() {
    let (dur, _dir) = fresh().await;
    refuse(
        &dur,
        "#!/bin/bash\n# placeholder\nexit 0",
        "placeholder_comment",
    )
    .await;
}

#[tokio::test]
async fn to_be_wired_phrase_refused() {
    let (dur, _dir) = fresh().await;
    refuse(
        &dur,
        "// to be wired in next PR\nfn handler() {}",
        "to_be_done",
    )
    .await;
}

#[tokio::test]
async fn not_yet_implemented_phrase_refused() {
    let (dur, _dir) = fresh().await;
    refuse(
        &dur,
        "# not yet implemented\ndef handler():\n    pass",
        "not_wired",
    )
    .await;
}

#[tokio::test]
async fn not_hooked_up_phrase_refused() {
    let (dur, _dir) = fresh().await;
    refuse(&dur, "// not hooked up\nfn handler() {}", "not_wired").await;
}

#[tokio::test]
async fn skeleton_marker_refused_anywhere() {
    let (dur, _dir) = fresh().await;
    refuse(&dur, "fn router() { /* (skeleton) */ }", "skeleton_marker").await;
}

#[tokio::test]
async fn rust_unreachable_refused() {
    let (dur, _dir) = fresh().await;
    refuse(&dur, "fn x(n: i32) { unreachable!() }", "rust_unreachable").await;
}

#[tokio::test]
async fn python_raise_not_implemented_refused() {
    let (dur, _dir) = fresh().await;
    refuse(
        &dur,
        "def handler():\n    raise NotImplementedError",
        "py_not_implemented",
    )
    .await;
}

#[tokio::test]
async fn java_not_implemented_exception_refused() {
    let (dur, _dir) = fresh().await;
    refuse(
        &dur,
        "public void run() {\n    throw new NotImplementedException();\n}",
        "jvm_not_implemented",
    )
    .await;
}

#[tokio::test]
async fn java_unsupported_op_refused() {
    let (dur, _dir) = fresh().await;
    refuse(
        &dur,
        "public void run() {\n    throw new UnsupportedOperationException(\"todo\");\n}",
        "jvm_unsupported_op",
    )
    .await;
}

#[tokio::test]
async fn clean_diverse_diffs_pass() {
    let (dur, _dir) = fresh().await;
    let cases: &[&str] = &[
        "fn add(a: i32, b: i32) -> i32 { a + b }",
        "def transform(items):\n    return [normalize(i) for i in items]",
        "CREATE TABLE accounts (id INTEGER PRIMARY KEY, balance REAL NOT NULL);",
        "<button aria-label=\"Save\">Save</button>",
        ".btn { color: var(--brand); padding: 1rem; }",
        "#!/bin/bash\nset -euo pipefail\nrun_thing",
        // Java (legitimate code with the word `unreachable` in a comment but
        // never as a function call should still pass; we only catch the
        // function form.)
        "// branch is unreachable in v2 but kept for clarity\nreturn 0;",
    ];
    for diff in cases {
        pass(&dur, diff).await;
    }
}

#[tokio::test]
async fn no_op_for_in_progress_target() {
    let (dur, _dir) = fresh().await;
    let task = task_with_diff(&dur, "// stub\nfn x() {}").await;
    let in_progress_ctx = GateContext {
        pool: dur.pool().clone(),
        task,
        target_status: TaskStatus::InProgress,
        agent_id: None,
    };
    NoStubGate::default().check(&in_progress_ctx).await.unwrap();
}

#[tokio::test]
async fn fires_through_full_facade_pipeline() {
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
        json!({"diff": "// placeholder\nfn handler() {}"}),
        Some(0),
    )
    .await
    .unwrap();

    let err = dur
        .transition_task(&task.id, TaskStatus::Submitted, Some("a"))
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("no_stub"), "msg: {msg}");
    assert!(msg.contains("placeholder_comment"), "msg: {msg}");
}
