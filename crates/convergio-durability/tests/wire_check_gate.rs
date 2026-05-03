//! Tests for `WireCheckGate` (F55-A — structural verification of
//! claimed routes / CLI paths).
//!
//! Each test sets `CONVERGIO_WIRE_CHECK_ROOT` to the real workspace
//! root (computed from `CARGO_MANIFEST_DIR`) so the gate scans the
//! actual `crates/` tree, not whatever cwd the test runner picked.
//! The env-var dance also exercises the gate's "missing workspace =
//! silent pass" branch (see `passes_when_workspace_root_missing`).
//!
//! `tokio::test` runs each function on its own runtime, but they
//! share the **process** environment — Rust's `set_var` is racy
//! across threads. We serialise env mutation with a `Mutex`.

use convergio_db::Pool;
use convergio_durability::gates::{Gate, GateContext, WireCheckGate};
use convergio_durability::{init, Durability, NewPlan, NewTask, TaskStatus};
use serde_json::json;
use std::path::PathBuf;
use std::sync::OnceLock;
use tempfile::tempdir;
use tokio::sync::Mutex;

/// Serialise env-var mutation across parallel tokio tests.
/// `tokio::sync::Mutex` so the guard can be safely held across
/// `.await` points (clippy refuses `std::sync::Mutex` here).
fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn workspace_root() -> PathBuf {
    // `CARGO_MANIFEST_DIR` points at this crate's directory at compile
    // time; the workspace root is two levels up (`../..`).
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(|p| p.parent())
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest.clone())
}

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
    if !payload.is_null() {
        dur.attach_evidence(&task.id, kind, payload, Some(0))
            .await
            .unwrap();
    }
    dur.tasks().get(&task.id).await.unwrap()
}

async fn bare_task(dur: &Durability) -> convergio_durability::Task {
    task_with_evidence(dur, "noop", serde_json::Value::Null).await
}

fn ctx(dur: &Durability, task: convergio_durability::Task) -> GateContext {
    GateContext {
        pool: dur.pool().clone(),
        task,
        target_status: TaskStatus::Submitted,
        agent_id: None,
    }
}

/// Set `CONVERGIO_WIRE_CHECK_ROOT` to the real workspace, run the
/// gate, then restore the previous value. The lock guards the
/// process-global env table.
async fn run_with_root(gate_ctx: &GateContext, root: Option<PathBuf>) -> Result<(), String> {
    let _guard = env_lock().lock().await;
    let prev = std::env::var("CONVERGIO_WIRE_CHECK_ROOT").ok();
    match root {
        // SAFETY: env mutation is serialised by ENV_LOCK above.
        Some(p) => unsafe { std::env::set_var("CONVERGIO_WIRE_CHECK_ROOT", p) },
        None => unsafe { std::env::remove_var("CONVERGIO_WIRE_CHECK_ROOT") },
    }
    let result = WireCheckGate
        .check(gate_ctx)
        .await
        .map_err(|e| e.to_string());
    match prev {
        Some(v) => unsafe { std::env::set_var("CONVERGIO_WIRE_CHECK_ROOT", v) },
        None => unsafe { std::env::remove_var("CONVERGIO_WIRE_CHECK_ROOT") },
    }
    result
}

#[tokio::test]
async fn passes_when_no_wire_check_evidence_attached() {
    let (dur, _dir) = fresh().await;
    // Task has *some* unrelated evidence but no `wire_check` row.
    let task = task_with_evidence(&dur, "code", json!({"diff": "fn ok() {}"})).await;
    let result = run_with_root(&ctx(&dur, task), Some(workspace_root())).await;
    assert!(result.is_ok(), "expected pass, got: {result:?}");
}

#[tokio::test]
async fn passes_when_claimed_route_actually_mounted() {
    let (dur, _dir) = fresh().await;
    let task = task_with_evidence(
        &dur,
        "wire_check",
        json!({
            "routes": [{"method": "GET", "path": "/v1/agent-registry/agents"}],
        }),
    )
    .await;
    let result = run_with_root(&ctx(&dur, task), Some(workspace_root())).await;
    assert!(result.is_ok(), "expected pass, got: {result:?}");
}

#[tokio::test]
async fn refuses_when_claimed_route_missing() {
    let (dur, _dir) = fresh().await;
    let task = task_with_evidence(
        &dur,
        "wire_check",
        json!({
            "routes": [{"method": "GET", "path": "/v1/totally-fake/path"}],
        }),
    )
    .await;
    let err = run_with_root(&ctx(&dur, task), Some(workspace_root()))
        .await
        .expect_err("expected refusal");
    assert!(err.contains("wire_check"), "msg: {err}");
    assert!(err.contains("/v1/totally-fake/path"), "msg: {err}");
    assert!(err.contains("route not mounted"), "msg: {err}");
}

#[tokio::test]
async fn refuses_when_claimed_cli_path_missing() {
    let (dur, _dir) = fresh().await;
    let task = task_with_evidence(
        &dur,
        "wire_check",
        json!({
            "cli_paths": ["banana split"],
        }),
    )
    .await;
    let err = run_with_root(&ctx(&dur, task), Some(workspace_root()))
        .await
        .expect_err("expected refusal");
    assert!(err.contains("wire_check"), "msg: {err}");
    assert!(err.contains("banana split"), "msg: {err}");
    assert!(err.contains("cli path not found"), "msg: {err}");
}

#[tokio::test]
async fn passes_when_cli_path_exists_in_main() {
    let (dur, _dir) = fresh().await;
    // `plan list` is shipped in main: see crates/convergio-cli/src/commands/plan.rs.
    let task = task_with_evidence(
        &dur,
        "wire_check",
        json!({
            "cli_paths": ["plan list"],
        }),
    )
    .await;
    let result = run_with_root(&ctx(&dur, task), Some(workspace_root())).await;
    assert!(result.is_ok(), "expected pass, got: {result:?}");
}

#[tokio::test]
async fn no_op_for_in_progress_target() {
    let (dur, _dir) = fresh().await;
    let task = task_with_evidence(
        &dur,
        "wire_check",
        json!({"routes": [{"method": "GET", "path": "/v1/totally-fake/path"}]}),
    )
    .await;
    let in_progress_ctx = GateContext {
        pool: dur.pool().clone(),
        task,
        target_status: TaskStatus::InProgress,
        agent_id: None,
    };
    let result = run_with_root(&in_progress_ctx, Some(workspace_root())).await;
    assert!(result.is_ok(), "in-progress should bypass: {result:?}");
}

#[tokio::test]
async fn passes_when_workspace_root_missing() {
    let (dur, _dir) = fresh().await;
    // Even an obviously-broken claim must not refuse when the
    // configured root is not a workspace (no `crates/` dir).
    let task = task_with_evidence(
        &dur,
        "wire_check",
        json!({"routes": [{"method": "GET", "path": "/v1/totally-fake/path"}]}),
    )
    .await;
    let scratch = tempdir().unwrap();
    let result = run_with_root(&ctx(&dur, task), Some(scratch.path().to_path_buf())).await;
    assert!(
        result.is_ok(),
        "expected silent pass on non-workspace root: {result:?}"
    );
}

#[tokio::test]
async fn empty_payload_is_silent_pass() {
    let (dur, _dir) = fresh().await;
    // Attach a wire_check row with both keys empty / missing.
    let task = task_with_evidence(&dur, "wire_check", json!({})).await;
    let result = run_with_root(&ctx(&dur, task), Some(workspace_root())).await;
    assert!(result.is_ok(), "expected pass, got: {result:?}");
}

#[tokio::test]
async fn unrelated_task_with_no_evidence_passes() {
    let (dur, _dir) = fresh().await;
    let task = bare_task(&dur).await;
    let result = run_with_root(&ctx(&dur, task), Some(workspace_root())).await;
    assert!(result.is_ok(), "expected pass, got: {result:?}");
}
