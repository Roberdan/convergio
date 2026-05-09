//! Basic tick behaviour tests.

mod support;

use convergio_durability::TaskStatus;
use support::fresh;

#[tokio::test]
async fn tick_dispatches_pending_tasks_in_first_wave() {
    let (exec, dur, _dir) = fresh().await;
    let plan = dur
        .create_plan(convergio_durability::NewPlan {
            title: "p".into(),
            description: None,
            project: None,
        })
        .await
        .unwrap();
    for seq in 1..=3 {
        dur.create_task(
            &plan.id,
            convergio_durability::NewTask {
                wave: 1,
                sequence: seq,
                title: format!("t{seq}"),
                description: None,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();
    }

    let dispatched = exec.tick().await.unwrap();
    assert_eq!(dispatched, 3);

    let tasks = dur.tasks().list_by_plan(&plan.id).await.unwrap();
    assert!(tasks.iter().all(|t| t.status == TaskStatus::InProgress));
    assert!(tasks.iter().all(|t| t.agent_id.is_some()));
}
