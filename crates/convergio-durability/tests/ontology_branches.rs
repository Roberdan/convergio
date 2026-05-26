//! Ontology branches: CoW overlay read/write semantics.

use convergio_db::Pool;
use convergio_durability::{init, Durability, OntologyBranchStatus, OntologyValueSource};
use tempfile::tempdir;
use tokio::time::{sleep, Duration};

async fn fresh() -> (Durability, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let url = format!("sqlite://{}/state.db", dir.path().display());
    let pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    (Durability::new(pool), dir)
}

#[tokio::test]
async fn branch_writes_do_not_touch_main_until_merge() {
    let (dur, _dir) = fresh().await;

    dur.set_ontology_entry("k1", serde_json::json!(1), None, None)
        .await
        .unwrap();

    let branch = dur.create_ontology_branch("b1", None).await.unwrap();

    dur.set_ontology_entry("k1", serde_json::json!(2), Some(&branch.id), None)
        .await
        .unwrap();

    let main = dur.resolve_ontology_entry("k1", None).await.unwrap();
    assert_eq!(main.value, serde_json::json!(1));
    assert_eq!(main.source, OntologyValueSource::Main);

    let overlay = dur
        .resolve_ontology_entry("k1", Some(&branch.id))
        .await
        .unwrap();
    assert_eq!(overlay.value, serde_json::json!(2));
    assert_eq!(overlay.source, OntologyValueSource::Branch);
}

#[tokio::test]
async fn branch_delete_shadows_main_but_does_not_delete_main() {
    let (dur, _dir) = fresh().await;

    dur.set_ontology_entry("k1", serde_json::json!("main"), None, None)
        .await
        .unwrap();

    let branch = dur.create_ontology_branch("b1", None).await.unwrap();
    dur.delete_ontology_entry("k1", Some(&branch.id), None)
        .await
        .unwrap();

    let main = dur.resolve_ontology_entry("k1", None).await.unwrap();
    assert_eq!(main.value, serde_json::json!("main"));
    assert_eq!(main.source, OntologyValueSource::Main);

    let overlay = dur
        .resolve_ontology_entry("k1", Some(&branch.id))
        .await
        .unwrap();
    assert!(overlay.value.is_null());
    assert_eq!(overlay.source, OntologyValueSource::None);
}

#[tokio::test]
async fn merge_applies_overlay_to_main_and_closes_branch() {
    let (dur, _dir) = fresh().await;

    dur.set_ontology_entry("k1", serde_json::json!(1), None, None)
        .await
        .unwrap();

    let branch = dur.create_ontology_branch("b1", None).await.unwrap();
    dur.set_ontology_entry("k1", serde_json::json!(3), Some(&branch.id), None)
        .await
        .unwrap();

    // Must go through review first.
    dur.transition_ontology_branch(&branch.id, OntologyBranchStatus::Review, None)
        .await
        .unwrap();

    dur.transition_ontology_branch(&branch.id, OntologyBranchStatus::Merged, None)
        .await
        .unwrap();

    let main = dur.resolve_ontology_entry("k1", None).await.unwrap();
    assert_eq!(main.value, serde_json::json!(3));

    let err = dur
        .set_ontology_entry("k2", serde_json::json!("x"), Some(&branch.id), None)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("writes are not allowed"));
}

#[tokio::test]
async fn illegal_transition_is_refused() {
    let (dur, _dir) = fresh().await;

    let branch = dur.create_ontology_branch("b1", None).await.unwrap();
    let err = dur
        .transition_ontology_branch(&branch.id, OntologyBranchStatus::Merged, None)
        .await
        .unwrap_err();
    assert!(err
        .to_string()
        .contains("illegal ontology branch transition"));
}

#[tokio::test]
async fn branch_overlay_writes_update_branch_updated_at() {
    let (dur, _dir) = fresh().await;

    let branch = dur.create_ontology_branch("b1", None).await.unwrap();
    let before = branch.updated_at;

    sleep(Duration::from_millis(2)).await;
    dur.set_ontology_entry("k1", serde_json::json!(1), Some(&branch.id), None)
        .await
        .unwrap();

    let updated = dur
        .list_ontology_branches()
        .await
        .unwrap()
        .into_iter()
        .find(|b| b.id == branch.id)
        .unwrap();

    assert!(updated.updated_at > before);
}
