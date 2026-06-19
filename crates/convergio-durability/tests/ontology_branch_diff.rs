//! Ontology branch diff + merge-as-plan generator (ADR-0056, W4).

use convergio_db::Pool;
use convergio_durability::{init, BranchChange, Durability, MergeOpKind};
use serde_json::json;

async fn fresh() -> (Durability, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}/state.db", dir.path().display());
    let pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    (Durability::new(pool), dir)
}

async fn set_base(dur: &Durability, key: &str, v: serde_json::Value) {
    dur.set_ontology_entry(key, v, None, None).await.unwrap();
}

async fn set_branch(dur: &Durability, b: &str, key: &str, v: serde_json::Value) {
    dur.set_ontology_entry(key, v, Some(b), None).await.unwrap();
}

async fn del_branch(dur: &Durability, b: &str, key: &str) {
    dur.delete_ontology_entry(key, Some(b), None).await.unwrap();
}

#[tokio::test]
async fn diff_classifies_added_modified_removed_in_key_order() {
    let (dur, _dir) = fresh().await;
    set_base(&dur, "keep", json!("base-keep")).await;
    set_base(&dur, "drop", json!("base-drop")).await;
    let b = dur.create_ontology_branch("scenario", None).await.unwrap();
    set_branch(&dur, &b.id, "keep", json!("branch-keep")).await; // Modified
    set_branch(&dur, &b.id, "add", json!("branch-add")).await; // Added
    del_branch(&dur, &b.id, "drop").await; // Removed

    let diff = dur.diff_ontology_branch(&b.id).await.unwrap();
    assert_eq!(diff.branch_id, b.id);
    let keys: Vec<&str> = diff.entries.iter().map(|e| e.key.as_str()).collect();
    assert_eq!(keys, vec!["add", "drop", "keep"]); // deterministic order

    assert_eq!(diff.entries[0].change, BranchChange::Added);
    assert_eq!(diff.entries[0].base, None);
    assert_eq!(diff.entries[0].branch, Some(json!("branch-add")));
    assert_eq!(diff.entries[1].change, BranchChange::Removed);
    assert_eq!(diff.entries[1].base, Some(json!("base-drop")));
    assert_eq!(diff.entries[1].branch, None);
    assert_eq!(diff.entries[2].change, BranchChange::Modified);
    assert_eq!(diff.entries[2].base, Some(json!("base-keep")));
    assert_eq!(diff.entries[2].branch, Some(json!("branch-keep")));
}

#[tokio::test]
async fn diff_omits_unchanged_override_and_tombstone_over_nothing() {
    let (dur, _dir) = fresh().await;
    set_base(&dur, "same", json!({"a": 1})).await;
    let b = dur.create_ontology_branch("noop", None).await.unwrap();
    set_branch(&dur, &b.id, "same", json!({"a": 1})).await; // identical: no change
    del_branch(&dur, &b.id, "ghost").await; // tombstone over nothing: no-op

    let diff = dur.diff_ontology_branch(&b.id).await.unwrap();
    assert!(diff.entries.is_empty(), "expected empty, got {diff:?}");
}

#[tokio::test]
async fn merge_plan_is_ordered_set_and_unset_ops() {
    let (dur, _dir) = fresh().await;
    set_base(&dur, "keep", json!(1)).await;
    set_base(&dur, "drop", json!(2)).await;
    let b = dur.create_ontology_branch("plan", None).await.unwrap();
    set_branch(&dur, &b.id, "keep", json!(9)).await;
    set_branch(&dur, &b.id, "add", json!(3)).await;
    del_branch(&dur, &b.id, "drop").await;

    let plan = dur.branch_merge_as_plan(&b.id).await.unwrap();
    assert_eq!(plan.branch_id, b.id);
    let keys: Vec<&str> = plan.ops.iter().map(|o| o.key.as_str()).collect();
    assert_eq!(keys, vec!["add", "drop", "keep"]);
    assert_eq!(plan.ops[0].op, MergeOpKind::Set);
    assert_eq!(plan.ops[0].value, Some(json!(3)));
    assert_eq!(plan.ops[1].op, MergeOpKind::Unset);
    assert_eq!(plan.ops[1].value, None);
    assert_eq!(plan.ops[2].op, MergeOpKind::Set);
    assert_eq!(plan.ops[2].value, Some(json!(9)));
}

#[tokio::test]
async fn empty_branch_and_unknown_branch() {
    let (dur, _dir) = fresh().await;
    let b = dur.create_ontology_branch("empty", None).await.unwrap();
    assert!(dur
        .diff_ontology_branch(&b.id)
        .await
        .unwrap()
        .entries
        .is_empty());
    assert!(dur
        .branch_merge_as_plan(&b.id)
        .await
        .unwrap()
        .ops
        .is_empty());

    let e = dur.diff_ontology_branch("nope").await.unwrap_err();
    assert!(e.to_string().contains("not found"));
    let e = dur.branch_merge_as_plan("nope").await.unwrap_err();
    assert!(e.to_string().contains("not found"));
}

#[tokio::test]
async fn merge_plan_applied_makes_base_equal_branch() {
    let (dur, _dir) = fresh().await;
    set_base(&dur, "keep", json!("old")).await;
    set_base(&dur, "drop", json!("gone")).await;
    let b = dur.create_ontology_branch("apply", None).await.unwrap();
    set_branch(&dur, &b.id, "keep", json!("new")).await;
    set_branch(&dur, &b.id, "add", json!("fresh")).await;
    del_branch(&dur, &b.id, "drop").await;

    let plan = dur.branch_merge_as_plan(&b.id).await.unwrap();
    for op in &plan.ops {
        match op.op {
            MergeOpKind::Set => set_base(&dur, &op.key, op.value.clone().unwrap()).await,
            MergeOpKind::Unset => dur
                .delete_ontology_entry(&op.key, None, None)
                .await
                .unwrap(),
        }
    }
    for key in ["keep", "add", "drop"] {
        let base = dur.resolve_ontology_entry(key, None).await.unwrap();
        let overlay = dur.resolve_ontology_entry(key, Some(&b.id)).await.unwrap();
        assert_eq!(base.value, overlay.value, "key {key} mismatch after plan");
    }
    // Post-merge re-diff is empty.
    assert!(dur
        .diff_ontology_branch(&b.id)
        .await
        .unwrap()
        .entries
        .is_empty());
}
