//! Integration tests for ontology draft import (ADR-0080 loop close).

use convergio_db::Pool;
use convergio_ontology::{import_draft, ImportDraft};

async fn migrated_store() -> anyhow::Result<(tempfile::TempDir, convergio_ontology::Store)> {
    let dir = tempfile::tempdir()?;
    let url = format!("sqlite://{}?mode=rwc", dir.path().join("o.db").display());
    let pool = Pool::connect(&url).await?;
    let store = convergio_ontology::Store::new(pool);
    store.migrate().await?;
    Ok((dir, store))
}

const DRAFT: &str = r#"{
  "name": "sis",
  "objects": [
    {"name": "Student", "title": "Student", "description": "A learner"},
    {"name": "Course", "title": "Course", "description": "A unit of study"}
  ],
  "properties": [
    {"name": "email", "owner": "Student", "datatype": "string", "required": true, "title": "Email"}
  ],
  "links": [
    {"name": "enrolled_in", "from": "Student", "to": "Course", "title": "Enrolled in"}
  ]
}"#;

#[tokio::test]
async fn imports_a_self_contained_draft() -> anyhow::Result<()> {
    let (_dir, store) = migrated_store().await?;
    let draft: ImportDraft = serde_json::from_str(DRAFT)?;

    let report = import_draft(&store, &draft).await?;
    assert_eq!(report.objects, 2);
    assert_eq!(report.properties, 1);
    assert_eq!(report.links, 1);

    let objects = store.list_objects().await?;
    assert_eq!(objects.len(), 2);
    let links = store.list_links().await?;
    assert_eq!(links.len(), 1);
    Ok(())
}

#[tokio::test]
async fn import_is_idempotent() -> anyhow::Result<()> {
    let (_dir, store) = migrated_store().await?;
    let draft: ImportDraft = serde_json::from_str(DRAFT)?;

    import_draft(&store, &draft).await?;
    let second = import_draft(&store, &draft).await?;
    assert_eq!(second.objects, 2);
    assert_eq!(store.list_objects().await?.len(), 2);
    Ok(())
}

#[tokio::test]
async fn rejects_duplicate_object_name() -> anyhow::Result<()> {
    let (_dir, store) = migrated_store().await?;
    let mut draft: ImportDraft = serde_json::from_str(DRAFT)?;
    draft.objects.push(draft.objects[0].clone());

    let err = import_draft(&store, &draft).await.unwrap_err();
    assert!(err.to_string().contains("duplicate"));
    assert_eq!(store.list_objects().await?.len(), 0);
    Ok(())
}

#[tokio::test]
async fn rejects_dangling_reference() -> anyhow::Result<()> {
    let (_dir, store) = migrated_store().await?;
    let mut draft: ImportDraft = serde_json::from_str(DRAFT)?;
    draft.links[0].to = "Ghost".to_string();

    let err = import_draft(&store, &draft).await.unwrap_err();
    assert!(err.to_string().contains("Ghost"));
    // Nothing partially committed beyond objects is not asserted here;
    // the closure check runs before any write.
    assert_eq!(store.list_objects().await?.len(), 0);
    Ok(())
}
