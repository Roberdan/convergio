//! Tests for `A11yGate` phase 1.

use convergio_db::Pool;
use convergio_durability::gates::{A11yGate, Gate, GateContext};
use convergio_durability::{init, Durability, DurabilityError, NewPlan, NewTask, TaskStatus};
use serde_json::{json, Value};
use tempfile::tempdir;

async fn fresh() -> (Durability, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let url = format!("sqlite://{}/state.db", dir.path().display());
    let pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    (Durability::new(pool), dir)
}

async fn make_task_with(
    dur: &Durability,
    kind: &str,
    payload: Value,
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
    dur.attach_evidence(&task.id, kind, payload, Some(0))
        .await
        .unwrap();
    dur.tasks().get(&task.id).await.unwrap()
}

fn ctx(dur: &Durability, task: convergio_durability::Task, target: TaskStatus) -> GateContext {
    GateContext {
        pool: dur.pool().clone(),
        task,
        target_status: target,
        agent_id: None,
    }
}

async fn assert_refused_with(kind: &str, payload: Value, expected_rule: &str) {
    let (dur, _dir) = fresh().await;
    let task = make_task_with(&dur, kind, payload).await;
    let err = A11yGate::default()
        .check(&ctx(&dur, task, TaskStatus::Submitted))
        .await
        .unwrap_err();
    match err {
        DurabilityError::GateRefused { gate, reason } => {
            assert_eq!(gate, "a11y", "wrong gate name");
            assert!(
                reason.contains(expected_rule),
                "expected `{expected_rule}` in `{reason}`"
            );
        }
        other => panic!("expected GateRefused, got {other:?}"),
    }
}

#[tokio::test]
async fn passes_on_clean_markdown() {
    let (dur, _dir) = fresh().await;
    let task = make_task_with(
        &dur,
        "markdown_doc",
        json!({"body": "# Title\n\n## Section\n\n![team photo](photo.png)\n\nSee the [installation guide](./install.md)."}),
    )
    .await;
    A11yGate::default()
        .check(&ctx(&dur, task, TaskStatus::Submitted))
        .await
        .unwrap();
}

#[tokio::test]
async fn no_op_for_in_progress_target() {
    let (dur, _dir) = fresh().await;
    let task = make_task_with(
        &dur,
        "markdown_doc",
        json!({"body": "# A\n\n### Skip\n\n![](x.png)"}),
    )
    .await;
    A11yGate::default()
        .check(&ctx(&dur, task, TaskStatus::InProgress))
        .await
        .unwrap();
}

#[tokio::test]
async fn refuses_each_built_in_rule_family() {
    // One row per rule. Adding a check? Add a row here.
    let cases: &[(&str, &str, Value)] = &[
        (
            "markdown_doc",
            "md_heading_skip",
            json!({"body": "# A\n\n### C jumps a level"}),
        ),
        (
            "markdown_doc",
            "md_image_missing_alt",
            json!({"body": "before ![](broken.png) after"}),
        ),
        (
            "markdown_doc",
            "md_link_nondescriptive",
            json!({"body": "see [click here](./install.md) for setup"}),
        ),
        (
            "markdown_doc",
            "md_color_only_emphasis",
            json!({"body": "<font color=\"red\">danger</font>"}),
        ),
        (
            "cli_output",
            "cli_color_only_signal",
            json!({"line": "\u{1b}[31m\u{1b}[0m"}),
        ),
        (
            "markdown_doc",
            "bidi_override",
            json!({"body": "innocent\u{202E}filename"}),
        ),
    ];
    for (kind, rule, payload) in cases {
        assert_refused_with(kind, payload.clone(), rule).await;
    }
}

#[tokio::test]
async fn ignores_markdown_rules_on_non_markdown_kind() {
    // The `code` kind is not in is_markdown_kind, so a missing-alt
    // image in code should not flag md_image_missing_alt. Bidi still
    // applies to every kind — keep this payload bidi-clean.
    let (dur, _dir) = fresh().await;
    let task = make_task_with(&dur, "code", json!({"diff": "let x = \"![](url)\";"})).await;
    A11yGate::default()
        .check(&ctx(&dur, task, TaskStatus::Submitted))
        .await
        .unwrap();
}

#[tokio::test]
async fn surfaces_evidence_kind_in_reason() {
    let (dur, _dir) = fresh().await;
    let task = make_task_with(&dur, "markdown_doc", json!({"body": "# T\n\n#### too far"})).await;
    let err = A11yGate::default()
        .check(&ctx(&dur, task, TaskStatus::Done))
        .await
        .unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("markdown_doc#md_heading_skip"),
        "expected kind#rule in reason, got: {msg}"
    );
}
