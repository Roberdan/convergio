//! Smart Thor (T3.02, ADR-0052) tests: built-in cargo:auto recipe,
//! `pipeline.run` audit row, and skip-when-trusted via the
//! `pipeline_run` evidence kind.

use convergio_db::Pool;
use convergio_durability::{init, Durability, NewPlan, NewTask, TaskStatus};
use convergio_thor::{Thor, Verdict};
use std::sync::Mutex;
use std::time::Duration;
use tempfile::tempdir;

// Tests that mutate CONVERGIO_THOR_WORKTREE_REV must serialize through
// this lock — cargo runs integration tests in parallel and env mutation
// is process-global.
static ENV_LOCK: Mutex<()> = Mutex::new(());

async fn fresh() -> (Durability, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let url = format!("sqlite://{}/state.db", dir.path().display());
    let pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    (Durability::new(pool), dir)
}

async fn submitted_plan_with_evidence(
    dur: &Durability,
    evidence_kinds: &[&str],
) -> (String, String) {
    let plan = dur
        .create_plan(NewPlan {
            title: "p".into(),
            description: None,
            project: None,
            no_dispatch_default: false,
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
                evidence_required: evidence_kinds.iter().map(|s| s.to_string()).collect(),
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
                no_dispatch: None,
            },
        )
        .await
        .unwrap();
    dur.transition_task(&task.id, TaskStatus::InProgress, Some("a"))
        .await
        .unwrap();
    for kind in evidence_kinds {
        dur.evidence()
            .attach(&task.id, kind, serde_json::json!({}), Some(0))
            .await
            .unwrap();
    }
    dur.transition_task(&task.id, TaskStatus::Submitted, Some("a"))
        .await
        .unwrap();
    (plan.id, task.id)
}

#[tokio::test]
async fn pipeline_run_audit_row_is_emitted_on_pass() {
    let (dur, _dir) = fresh().await;
    let thor =
        Thor::with_pipeline_timeout(dur.clone(), Some("true".into()), Duration::from_secs(5));
    let (plan_id, _task_id) = submitted_plan_with_evidence(&dur, &[]).await;

    let verdict = thor.validate(&plan_id).await.unwrap();
    assert!(matches!(verdict, Verdict::Pass));

    let entries = dur.audit().list_since(0, 1000).await.unwrap();
    let mut found = false;
    for e in entries {
        if e.transition == "pipeline.run" && e.entity_id == plan_id {
            let payload: serde_json::Value = serde_json::from_str(&e.payload).unwrap();
            assert_eq!(payload["ok"], serde_json::json!(true));
            assert_eq!(payload["recipe"], serde_json::json!("shell"));
            found = true;
        }
    }
    assert!(found, "expected pipeline.run audit row");
}

#[tokio::test]
async fn pipeline_run_audit_row_is_emitted_on_fail() {
    let (dur, _dir) = fresh().await;
    let thor =
        Thor::with_pipeline_timeout(dur.clone(), Some("exit 17".into()), Duration::from_secs(5));
    let (plan_id, _task_id) = submitted_plan_with_evidence(&dur, &[]).await;

    let verdict = thor.validate(&plan_id).await.unwrap();
    assert!(matches!(verdict, Verdict::Fail { .. }));

    let entries = dur.audit().list_since(0, 1000).await.unwrap();
    let row = entries
        .into_iter()
        .find(|e| e.transition == "pipeline.run" && e.entity_id == plan_id)
        .expect("pipeline.run audit row missing on failure");
    let payload: serde_json::Value = serde_json::from_str(&row.payload).unwrap();
    assert_eq!(payload["ok"], serde_json::json!(false));
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn pipeline_skipped_when_evidence_marks_worktree_pre_validated() {
    let _guard = ENV_LOCK.lock().unwrap();
    let (dur, _dir) = fresh().await;
    let rev = "deadbeef-w3-test";
    std::env::set_var("CONVERGIO_THOR_WORKTREE_REV", rev);

    let thor = Thor::with_pipeline_timeout(
        dur.clone(),
        // A failing command — if Thor honors the skip, we never invoke it.
        Some("exit 99".into()),
        Duration::from_secs(5),
    );
    let (plan_id, task_id) = submitted_plan_with_evidence(&dur, &[]).await;
    dur.evidence()
        .attach(
            &task_id,
            "pipeline_run",
            serde_json::json!({ "worktree_rev": rev, "result": "pass" }),
            Some(0),
        )
        .await
        .unwrap();

    let verdict = thor.validate(&plan_id).await.unwrap();
    std::env::remove_var("CONVERGIO_THOR_WORKTREE_REV");

    assert!(
        matches!(verdict, Verdict::Pass),
        "pre-validated evidence must skip the pipeline; got {:?}",
        verdict
    );
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn pipeline_runs_when_worktree_rev_mismatches_evidence() {
    let _guard = ENV_LOCK.lock().unwrap();
    let (dur, _dir) = fresh().await;
    std::env::set_var("CONVERGIO_THOR_WORKTREE_REV", "fresh-rev");

    let thor =
        Thor::with_pipeline_timeout(dur.clone(), Some("exit 7".into()), Duration::from_secs(5));
    let (plan_id, task_id) = submitted_plan_with_evidence(&dur, &[]).await;
    dur.evidence()
        .attach(
            &task_id,
            "pipeline_run",
            serde_json::json!({ "worktree_rev": "stale-rev", "result": "pass" }),
            Some(0),
        )
        .await
        .unwrap();

    let verdict = thor.validate(&plan_id).await.unwrap();
    std::env::remove_var("CONVERGIO_THOR_WORKTREE_REV");

    match verdict {
        Verdict::Fail { reasons } => {
            assert!(reasons.iter().any(|r| r.contains("pipeline_refused")));
        }
        Verdict::Pass => panic!("stale worktree_rev must not short-circuit pipeline"),
    }
}
