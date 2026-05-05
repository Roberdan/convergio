//! Tests for task templates: evidence_required pre-population per category.

use convergio_db::Pool;
use convergio_durability::{init, Durability, NewPlan, NewTask, TaskTemplate};
use tempfile::tempdir;

async fn fresh() -> (Durability, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let url = format!("sqlite://{}/state.db", dir.path().display());
    let pool: Pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    (Durability::new(pool), dir)
}

async fn make_plan(dur: &Durability) -> convergio_durability::Plan {
    dur.create_plan(NewPlan {
        title: "template test plan".into(),
        description: None,
        project: None,
    })
    .await
    .unwrap()
}

// ── unit: TaskTemplate::default_evidence ─────────────────────────────────────

#[test]
fn impl_template_defaults() {
    let ev = TaskTemplate::Impl.default_evidence();
    assert!(ev.contains(&"pr_link".to_string()));
    assert!(ev.contains(&"test_pass".to_string()));
}

#[test]
fn docs_template_defaults() {
    let ev = TaskTemplate::Docs.default_evidence();
    assert!(ev.contains(&"pr_link".to_string()));
    assert_eq!(ev.len(), 1, "docs only needs pr_link");
}

#[test]
fn refactor_template_defaults() {
    let ev = TaskTemplate::Refactor.default_evidence();
    assert!(ev.contains(&"pr_link".to_string()));
    assert!(ev.contains(&"test_pass".to_string()));
}

#[test]
fn test_template_defaults() {
    let ev = TaskTemplate::Test.default_evidence();
    assert!(ev.contains(&"pr_link".to_string()));
    assert!(ev.contains(&"test_pass".to_string()));
}

// ── integration: store applies template when evidence_required is empty ───────

#[tokio::test]
async fn store_applies_impl_template_when_evidence_empty() {
    let (dur, _dir) = fresh().await;
    let plan = make_plan(&dur).await;
    let task = dur
        .create_task(
            &plan.id,
            NewTask {
                wave: 1,
                sequence: 1,
                title: "impl task".into(),
                description: None,
                evidence_required: vec![],
                template: Some(TaskTemplate::Impl),
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();
    assert!(task.evidence_required.contains(&"pr_link".to_string()));
    assert!(task.evidence_required.contains(&"test_pass".to_string()));
}

#[tokio::test]
async fn store_applies_docs_template_when_evidence_empty() {
    let (dur, _dir) = fresh().await;
    let plan = make_plan(&dur).await;
    let task = dur
        .create_task(
            &plan.id,
            NewTask {
                wave: 1,
                sequence: 1,
                title: "docs task".into(),
                description: None,
                evidence_required: vec![],
                template: Some(TaskTemplate::Docs),
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(task.evidence_required, vec!["pr_link".to_string()]);
}

#[tokio::test]
async fn explicit_evidence_overrides_template() {
    // When evidence_required is non-empty, template defaults must NOT override.
    let (dur, _dir) = fresh().await;
    let plan = make_plan(&dur).await;
    let task = dur
        .create_task(
            &plan.id,
            NewTask {
                wave: 1,
                sequence: 1,
                title: "custom evidence task".into(),
                description: None,
                evidence_required: vec!["custom_kind".into()],
                template: Some(TaskTemplate::Impl),
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(task.evidence_required, vec!["custom_kind".to_string()]);
}

#[tokio::test]
async fn no_template_no_evidence_stays_empty() {
    let (dur, _dir) = fresh().await;
    let plan = make_plan(&dur).await;
    let task = dur
        .create_task(
            &plan.id,
            NewTask {
                wave: 1,
                sequence: 1,
                title: "bare task".into(),
                description: None,
                evidence_required: vec![],
                template: None,
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();
    assert!(task.evidence_required.is_empty());
}
