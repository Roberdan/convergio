//! Tracker-only / `no_dispatch` behaviour tests (plan A.2).
//!
//! Verifies that tasks flagged `no_dispatch = true` — whether
//! explicitly per-task or inherited from a plan-level
//! `no_dispatch_default` — stay `pending` across many executor
//! ticks and never get claimed, while sibling normal tasks in the
//! same plan are still dispatched on the first ready tick.

mod support;

use convergio_durability::{NewPlan, NewTask, TaskStatus};
use support::fresh;

#[tokio::test]
async fn no_dispatch_task_stays_pending_across_many_ticks() {
    let (exec, dur, _dir) = fresh().await;
    let plan = dur
        .create_plan(NewPlan {
            title: "tracker".into(),
            description: None,
            project: None,
            no_dispatch_default: false,
        })
        .await
        .unwrap();

    let tracker = dur
        .create_task(
            &plan.id,
            NewTask {
                wave: 1,
                sequence: 1,
                title: "mirrored from other repo".into(),
                description: None,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
                no_dispatch: Some(true),
            },
        )
        .await
        .unwrap();
    assert!(tracker.no_dispatch, "explicit no_dispatch must persist");

    // Many ticks; tracker stays pending and is never claimed.
    for _ in 0..10 {
        let dispatched = exec.tick().await.unwrap();
        assert_eq!(dispatched, 0, "tracker tasks must never be dispatched");
    }

    let reread = dur.tasks().get(&tracker.id).await.unwrap();
    assert_eq!(reread.status, TaskStatus::Pending);
    assert!(reread.agent_id.is_none());
    assert!(reread.no_dispatch);
}

#[tokio::test]
async fn no_dispatch_does_not_block_sibling_normal_task() {
    let (exec, dur, _dir) = fresh().await;
    let plan = dur
        .create_plan(NewPlan {
            title: "mixed".into(),
            description: None,
            project: None,
            no_dispatch_default: false,
        })
        .await
        .unwrap();

    let tracker = dur
        .create_task(
            &plan.id,
            NewTask {
                wave: 1,
                sequence: 1,
                title: "tracker".into(),
                description: None,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
                no_dispatch: Some(true),
            },
        )
        .await
        .unwrap();

    let normal = dur
        .create_task(
            &plan.id,
            NewTask {
                wave: 1,
                sequence: 2,
                title: "normal".into(),
                description: None,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
                no_dispatch: None,
            },
        )
        .await
        .unwrap();

    let dispatched = exec.tick().await.unwrap();
    assert_eq!(dispatched, 1, "only the normal task must be claimed");

    let tracker = dur.tasks().get(&tracker.id).await.unwrap();
    assert_eq!(tracker.status, TaskStatus::Pending);
    assert!(tracker.agent_id.is_none());

    let normal = dur.tasks().get(&normal.id).await.unwrap();
    assert_eq!(normal.status, TaskStatus::InProgress);
    assert!(normal.agent_id.is_some());
}

#[tokio::test]
async fn plan_no_dispatch_default_propagates_to_tasks() {
    let (exec, dur, _dir) = fresh().await;
    let plan = dur
        .create_plan(NewPlan {
            title: "tracker-plan".into(),
            description: None,
            project: None,
            no_dispatch_default: true,
        })
        .await
        .unwrap();
    assert!(plan.no_dispatch_default);

    // Body omits `no_dispatch` ⇒ inherit plan default.
    let inherited = dur
        .create_task(
            &plan.id,
            NewTask {
                wave: 1,
                sequence: 1,
                title: "inherits".into(),
                description: None,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
                no_dispatch: None,
            },
        )
        .await
        .unwrap();
    assert!(
        inherited.no_dispatch,
        "task without explicit no_dispatch must inherit plan default"
    );

    // Explicit `false` on the body overrides the plan default.
    let opted_in = dur
        .create_task(
            &plan.id,
            NewTask {
                wave: 1,
                sequence: 2,
                title: "explicit override".into(),
                description: None,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
                no_dispatch: Some(false),
            },
        )
        .await
        .unwrap();
    assert!(
        !opted_in.no_dispatch,
        "explicit no_dispatch=false on task body must win"
    );

    let dispatched = exec.tick().await.unwrap();
    assert_eq!(
        dispatched, 1,
        "only the explicitly-opted-in task is dispatched"
    );

    let inherited = dur.tasks().get(&inherited.id).await.unwrap();
    assert_eq!(inherited.status, TaskStatus::Pending);
    let opted_in = dur.tasks().get(&opted_in.id).await.unwrap();
    assert_eq!(opted_in.status, TaskStatus::InProgress);
}
