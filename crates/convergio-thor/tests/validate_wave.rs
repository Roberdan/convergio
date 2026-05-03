//! Integration tests for T3.06 — wave-scoped validation
//! ([`Thor::validate_wave`]). Kept in a sibling test file so the
//! main `validate.rs` stays under the 300-line cap enforced by the
//! file-size guard CI step.

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

async fn add_task_in_wave(
    dur: &Durability,
    plan_id: &str,
    wave: i64,
    seq: i64,
    title: &str,
    evidence_required: Vec<String>,
) -> String {
    let task = dur
        .create_task(
            plan_id,
            NewTask {
                wave,
                sequence: seq,
                title: title.into(),
                description: None,
                evidence_required,
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();
    task.id
}

async fn submit_with_evidence(dur: &Durability, task_id: &str, evidence_kind: &str) {
    dur.transition_task(task_id, TaskStatus::InProgress, Some("a"))
        .await
        .unwrap();
    if !evidence_kind.is_empty() {
        dur.attach_evidence(task_id, evidence_kind, json!({"ok": true}), None)
            .await
            .unwrap();
    }
    dur.transition_task(task_id, TaskStatus::Submitted, Some("a"))
        .await
        .unwrap();
}

#[tokio::test]
async fn validate_wave_promotes_only_named_wave() {
    // Wave 1 fully submitted with evidence; wave 2 has a pending
    // task. Plan-strict validate would fail. Wave-scoped validate
    // on wave 1 must Pass and promote wave-1 tasks; wave-2 task
    // must stay pending.
    let (thor, dur, _dir) = fresh().await;
    let plan = dur
        .create_plan(NewPlan {
            title: "wave-scoped".into(),
            description: None,
            project: None,
        })
        .await
        .unwrap();
    let t_w1_a = add_task_in_wave(&dur, &plan.id, 1, 1, "w1.a", vec!["doc".into()]).await;
    let t_w1_b = add_task_in_wave(&dur, &plan.id, 1, 2, "w1.b", vec!["doc".into()]).await;
    let _t_w2 = add_task_in_wave(&dur, &plan.id, 2, 1, "w2.a", vec![]).await;
    submit_with_evidence(&dur, &t_w1_a, "doc").await;
    submit_with_evidence(&dur, &t_w1_b, "doc").await;

    // Plan-strict fails because wave 2 has a pending task.
    let plain = thor.validate(&plan.id).await.unwrap();
    assert!(matches!(plain, Verdict::Fail { .. }));

    // Wave-scoped on wave 1 passes and promotes wave-1 tasks.
    let scoped = thor.validate_wave(&plan.id, Some(1)).await.unwrap();
    matches!(scoped, Verdict::Pass);
    let a = dur.tasks().get(&t_w1_a).await.unwrap();
    let b = dur.tasks().get(&t_w1_b).await.unwrap();
    assert_eq!(a.status, TaskStatus::Done);
    assert_eq!(b.status, TaskStatus::Done);

    // Wave 2 task untouched.
    let w2 = dur.tasks().get(&_t_w2).await.unwrap();
    assert_eq!(w2.status, TaskStatus::Pending);
}

#[tokio::test]
async fn validate_wave_fails_if_named_wave_has_pending_task() {
    let (thor, dur, _dir) = fresh().await;
    let plan = dur
        .create_plan(NewPlan {
            title: "wave-scoped-fail".into(),
            description: None,
            project: None,
        })
        .await
        .unwrap();
    let t_a = add_task_in_wave(&dur, &plan.id, 1, 1, "a", vec![]).await;
    let _t_b = add_task_in_wave(&dur, &plan.id, 1, 2, "b", vec![]).await;
    // a submitted; b stays pending.
    submit_with_evidence(&dur, &t_a, "").await;

    let v = thor.validate_wave(&plan.id, Some(1)).await.unwrap();
    assert!(matches!(v, Verdict::Fail { .. }));
    // a must NOT have been promoted.
    let a = dur.tasks().get(&t_a).await.unwrap();
    assert_eq!(a.status, TaskStatus::Submitted);
}

#[tokio::test]
async fn validate_wave_returns_fail_when_wave_has_no_tasks() {
    let (thor, dur, _dir) = fresh().await;
    let plan = dur
        .create_plan(NewPlan {
            title: "empty-wave".into(),
            description: None,
            project: None,
        })
        .await
        .unwrap();
    let _ = add_task_in_wave(&dur, &plan.id, 1, 1, "only-wave-1", vec![]).await;
    let v = thor.validate_wave(&plan.id, Some(99)).await.unwrap();
    let reasons = match v {
        Verdict::Fail { reasons } => reasons,
        Verdict::Pass => panic!("empty wave must fail, not pass"),
    };
    assert!(
        reasons.iter().any(|r| r.contains("no tasks in wave 99")),
        "expected 'no tasks in wave 99' reason, got {reasons:?}"
    );
}

#[tokio::test]
async fn validate_without_wave_keeps_plan_strict_behaviour() {
    // Backward compatibility: validate(plan_id) must behave exactly
    // as before — even one pending task in any wave fails.
    let (thor, dur, _dir) = fresh().await;
    let plan = dur
        .create_plan(NewPlan {
            title: "no-wave-flag".into(),
            description: None,
            project: None,
        })
        .await
        .unwrap();
    let t_w1 = add_task_in_wave(&dur, &plan.id, 1, 1, "w1", vec![]).await;
    let _t_w2 = add_task_in_wave(&dur, &plan.id, 2, 1, "w2", vec![]).await;
    submit_with_evidence(&dur, &t_w1, "").await;
    // w2 is pending.
    let v = thor.validate(&plan.id).await.unwrap();
    assert!(matches!(v, Verdict::Fail { .. }));
}
