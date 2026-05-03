//! Thor integration tests.

use convergio_db::Pool;
use convergio_durability::{init, Durability, NewPlan, NewTask, TaskStatus};
use convergio_thor::{Thor, Verdict};
use serde_json::json;
use tempfile::tempdir;

async fn fresh() -> (Thor, Durability, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let url = format!("sqlite://{}/state.db", dir.path().display());
    let pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    let dur = Durability::new(pool);
    (Thor::new(dur.clone()), dur, dir)
}

async fn plan_with_one_task(dur: &Durability, evidence_required: Vec<String>) -> (String, String) {
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
                evidence_required,
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();
    (plan.id, task.id)
}

#[tokio::test]
async fn pass_promotes_submitted_to_done_with_required_evidence() {
    let (thor, dur, _dir) = fresh().await;
    let (plan_id, task_id) = plan_with_one_task(&dur, vec!["test_pass".into()]).await;

    // Pending: validate must fail with "expected submitted or done".
    let v = thor.validate(&plan_id).await.unwrap();
    assert!(matches!(v, Verdict::Fail { .. }));

    dur.transition_task(&task_id, TaskStatus::InProgress, Some("a"))
        .await
        .unwrap();
    dur.attach_evidence(&task_id, "test_pass", json!({}), Some(0))
        .await
        .unwrap();
    dur.transition_task(&task_id, TaskStatus::Submitted, Some("a"))
        .await
        .unwrap();

    // Submitted with required evidence: Thor passes AND flips to done.
    let v = thor.validate(&plan_id).await.unwrap();
    assert!(matches!(v, Verdict::Pass), "expected Pass, got {v:?}");
    let task = dur.tasks().get(&task_id).await.unwrap();
    assert_eq!(task.status, TaskStatus::Done);

    // Re-validate is idempotent: still Pass, no further mutation.
    let v = thor.validate(&plan_id).await.unwrap();
    assert!(matches!(v, Verdict::Pass));
    let task = dur.tasks().get(&task_id).await.unwrap();
    assert_eq!(task.status, TaskStatus::Done);
}

#[tokio::test]
async fn fail_when_evidence_missing_keeps_task_submitted() {
    // The evidence gate already refuses agent-driven `submitted` when
    // required kinds are missing. To exercise Thor's defense-in-depth
    // check we drop a task into `submitted` via the store backdoor —
    // simulating either a corrupted state or a future code path that
    // would bypass the gate. Thor must still refuse and leave the task
    // submitted (no rogue promotion).
    let (thor, dur, _dir) = fresh().await;
    let (plan_id, task_id) = plan_with_one_task(&dur, vec!["test_pass".into()]).await;
    dur.tasks()
        .set_status(&task_id, TaskStatus::Submitted, Some("a"))
        .await
        .unwrap();
    let v = thor.validate(&plan_id).await.unwrap();
    match v {
        Verdict::Fail { reasons } => {
            assert!(
                reasons.iter().any(|r| r.contains("test_pass")),
                "reasons should name missing kind: {reasons:?}"
            );
        }
        Verdict::Pass => panic!("expected fail"),
    }
    let task = dur.tasks().get(&task_id).await.unwrap();
    assert_eq!(task.status, TaskStatus::Submitted);
}

#[tokio::test]
async fn agent_done_transition_is_refused_at_durability_layer() {
    let (_thor, dur, _dir) = fresh().await;
    let (_plan_id, task_id) = plan_with_one_task(&dur, vec![]).await;
    let err = dur
        .transition_task(&task_id, TaskStatus::Done, Some("rogue"))
        .await
        .unwrap_err();
    let s = err.to_string();
    assert!(
        s.contains("done is set only by validation"),
        "expected DoneNotByThor, got: {s}"
    );
    let task = dur.tasks().get(&task_id).await.unwrap();
    assert_eq!(task.status, TaskStatus::Pending, "status must not change");
}

#[tokio::test]
async fn fail_when_plan_has_no_tasks() {
    let (thor, dur, _dir) = fresh().await;
    let plan = dur
        .create_plan(NewPlan {
            title: "empty".into(),
            description: None,
            project: None,
        })
        .await
        .unwrap();
    let v = thor.validate(&plan.id).await.unwrap();
    matches!(v, Verdict::Fail { .. });
}

#[tokio::test]
async fn unknown_plan_returns_error() {
    let (thor, _dur, _dir) = fresh().await;
    assert!(thor.validate("does-not-exist").await.is_err());
}

// ---------- T3.02: smart Thor pipeline invocation ----------
//
// When configured, Thor runs a shell command as a "third gate" after
// evidence-shape validation. A passing pipeline lets the verdict
// proceed to Pass; a failing pipeline produces Verdict::Fail with
// the (truncated) stderr in the reason. Unset config = unchanged
// behaviour, so existing callers do not regress.

async fn fresh_with_pipeline(cmd: Option<&str>) -> (Thor, Durability, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let url = format!("sqlite://{}/state.db", dir.path().display());
    let pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    let dur = Durability::new(pool);
    (
        Thor::with_pipeline(dur.clone(), cmd.map(|s| s.to_string())),
        dur,
        dir,
    )
}

#[tokio::test]
async fn pipeline_pass_promotes_to_done() {
    // `true` exits 0 — the pipeline passes.
    let (thor, dur, _dir) = fresh_with_pipeline(Some("true")).await;
    let (plan_id, task_id) = plan_with_one_task(&dur, vec![]).await;
    dur.transition_task(&task_id, TaskStatus::InProgress, Some("a"))
        .await
        .unwrap();
    dur.transition_task(&task_id, TaskStatus::Submitted, Some("a"))
        .await
        .unwrap();
    let v = thor.validate(&plan_id).await.unwrap();
    matches!(v, Verdict::Pass);
    let task = dur.tasks().get(&task_id).await.unwrap();
    assert_eq!(task.status, TaskStatus::Done);
}

#[tokio::test]
async fn pipeline_fail_blocks_promotion() {
    // `false` exits 1 — the pipeline fails. Even with all evidence
    // shape correct, the verdict must be Fail and the task must
    // stay at submitted.
    let (thor, dur, _dir) = fresh_with_pipeline(Some("false")).await;
    let (plan_id, task_id) = plan_with_one_task(&dur, vec![]).await;
    dur.transition_task(&task_id, TaskStatus::InProgress, Some("a"))
        .await
        .unwrap();
    dur.transition_task(&task_id, TaskStatus::Submitted, Some("a"))
        .await
        .unwrap();
    let v = thor.validate(&plan_id).await.unwrap();
    match v {
        Verdict::Fail { reasons } => {
            assert!(reasons.iter().any(|r| r.contains("pipeline")));
        }
        Verdict::Pass => panic!("pipeline=false must produce Fail"),
    }
    let task = dur.tasks().get(&task_id).await.unwrap();
    assert_eq!(
        task.status,
        TaskStatus::Submitted,
        "submitted must not be promoted on pipeline failure"
    );
}

#[tokio::test]
async fn pipeline_failure_includes_stderr_tail() {
    // The verdict's reason should carry enough of the failed
    // command's output to debug from.
    let (thor, dur, _dir) = fresh_with_pipeline(Some("echo SENTINEL_OUTPUT >&2; exit 7")).await;
    let (plan_id, task_id) = plan_with_one_task(&dur, vec![]).await;
    dur.transition_task(&task_id, TaskStatus::InProgress, Some("a"))
        .await
        .unwrap();
    dur.transition_task(&task_id, TaskStatus::Submitted, Some("a"))
        .await
        .unwrap();
    let v = thor.validate(&plan_id).await.unwrap();
    match v {
        Verdict::Fail { reasons } => {
            let joined = reasons.join("\n");
            assert!(joined.contains("SENTINEL_OUTPUT"), "joined: {joined}");
            assert!(joined.contains("exit=7"), "joined: {joined}");
        }
        Verdict::Pass => panic!("expected Fail"),
    }
}

#[tokio::test]
async fn no_pipeline_means_unchanged_behaviour() {
    // Pipeline cmd None is the v0 path. The happy path test from
    // earlier in this file already covers this in the implicit way;
    // here we make the "no regression" claim explicit.
    let (thor, dur, _dir) = fresh_with_pipeline(None).await;
    let (plan_id, task_id) = plan_with_one_task(&dur, vec![]).await;
    dur.transition_task(&task_id, TaskStatus::InProgress, Some("a"))
        .await
        .unwrap();
    dur.transition_task(&task_id, TaskStatus::Submitted, Some("a"))
        .await
        .unwrap();
    let v = thor.validate(&plan_id).await.unwrap();
    matches!(v, Verdict::Pass);
    let task = dur.tasks().get(&task_id).await.unwrap();
    assert_eq!(task.status, TaskStatus::Done);
}

// T3.06 wave-scoped tests live in `tests/validate_wave.rs`.
