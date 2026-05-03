//! Multi-language coverage tests for `NoDebtGate`.
//!
//! Each test posts an evidence payload that mimics what an LLM agent
//! might attach in a typical language and asserts the gate refuses
//! with the right rule name.

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

async fn assert_refused_with(dur: &Durability, diff: &str, rule: &str) {
    let task = task_with_diff(dur, diff).await;
    let err = NoDebtGate::default()
        .check(&ctx(dur, task))
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains(rule), "expected rule `{rule}` in error: {msg}");
}

async fn assert_passes(dur: &Durability, diff: &str) {
    let task = task_with_diff(dur, diff).await;
    NoDebtGate::default()
        .check(&ctx(dur, task))
        .await
        .unwrap_or_else(|e| panic!("expected pass for diff `{diff}`, got error: {e}"));
}

// ----- Rust ---------------------------------------------------------

#[tokio::test]
async fn rust_unwrap_refused_but_unwrap_or_passes() {
    let (dur, _dir) = fresh().await;
    assert_refused_with(&dur, "let v = parse(s).unwrap();", "rust_unwrap").await;
    // unwrap_or, unwrap_or_default, unwrap_or_else must NOT match.
    assert_passes(&dur, "let v = parse(s).unwrap_or_default();").await;
}

#[tokio::test]
async fn rust_dbg_refused() {
    let (dur, _dir) = fresh().await;
    assert_refused_with(&dur, "dbg!(value);", "rust_dbg").await;
}

// ----- TypeScript ---------------------------------------------------

#[tokio::test]
async fn ts_as_any_refused() {
    let (dur, _dir) = fresh().await;
    assert_refused_with(&dur, "const x = (input as any).field;", "ts_as_any").await;
}

#[tokio::test]
async fn ts_ignore_refused() {
    let (dur, _dir) = fresh().await;
    assert_refused_with(
        &dur,
        "// @ts-ignore\nconst x: number = 'string';",
        "ts_ignore",
    )
    .await;
}

#[tokio::test]
async fn js_debugger_statement_refused() {
    let (dur, _dir) = fresh().await;
    assert_refused_with(&dur, "if (cond) { debugger; }", "js_debugger").await;
}

// ----- Python -------------------------------------------------------

#[tokio::test]
async fn py_pdb_set_trace_refused() {
    let (dur, _dir) = fresh().await;
    assert_refused_with(
        &dur,
        "import pdb\npdb.set_trace()\nresult = work()",
        "py_pdb_set_trace",
    )
    .await;
}

#[tokio::test]
async fn py_breakpoint_refused() {
    let (dur, _dir) = fresh().await;
    assert_refused_with(&dur, "if cond:\n    breakpoint()", "py_breakpoint").await;
}

#[tokio::test]
async fn py_ipdb_import_refused() {
    let (dur, _dir) = fresh().await;
    assert_refused_with(&dur, "from ipdb import set_trace", "py_ipdb").await;
}

// ----- Go -----------------------------------------------------------

#[tokio::test]
async fn go_panic_refused() {
    let (dur, _dir) = fresh().await;
    assert_refused_with(&dur, "if err != nil { panic(err) }", "go_panic").await;
}

#[tokio::test]
async fn go_blank_err_refused() {
    let (dur, _dir) = fresh().await;
    assert_refused_with(&dur, "x, _ := tryParse(s); _ = err", "go_blank_err").await;
}

#[tokio::test]
async fn go_nolint_comment_refused() {
    let (dur, _dir) = fresh().await;
    assert_refused_with(&dur, "var x int // nolint:unused", "go_nolint").await;
}

// ----- Swift --------------------------------------------------------

#[tokio::test]
async fn swift_fatal_error_refused() {
    let (dur, _dir) = fresh().await;
    assert_refused_with(
        &dur,
        "guard let v = obj else { fatalError(\"missing\") }",
        "swift_fatal_error",
    )
    .await;
}

#[tokio::test]
async fn swift_try_bang_refused() {
    let (dur, _dir) = fresh().await;
    assert_refused_with(&dur, "let result = try! risky()", "swift_try_bang").await;
}

// ----- Shell --------------------------------------------------------

#[tokio::test]
async fn shell_silent_errors_refused() {
    let (dur, _dir) = fresh().await;
    assert_refused_with(&dur, "#!/bin/bash\nset +e\nrm -rf /", "sh_silent_errors").await;
}

// ----- WIP marker ---------------------------------------------------

#[tokio::test]
async fn wip_marker_refused() {
    let (dur, _dir) = fresh().await;
    assert_refused_with(&dur, "// WIP: clean up later", "todo_marker").await;
}

// ----- Sanity: clean code in every language -------------------------

#[tokio::test]
async fn diverse_clean_code_passes() {
    let (dur, _dir) = fresh().await;
    let cases: &[&str] = &[
        // Rust
        "fn add(a: i32, b: i32) -> i32 { a + b }",
        "let value = parse(input).unwrap_or_else(|| default());",
        // TypeScript
        "const x: number = parseInt(s, 10);",
        // Python
        "def process(items): return [transform(i) for i in items]",
        // Go
        "if err != nil { return fmt.Errorf(\"context: %w\", err) }",
        // Swift
        "guard let v = obj else { throw MyError.notFound }",
        // Shell
        "#!/bin/bash\nset -euo pipefail\nrun_thing",
    ];
    for diff in cases {
        let task = task_with_diff(&dur, diff).await;
        NoDebtGate::default()
            .check(&ctx(&dur, task))
            .await
            .unwrap_or_else(|e| panic!("expected clean: `{diff}`, got: {e}"));
    }
}
