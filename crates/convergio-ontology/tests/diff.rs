//! Integration tests for the deterministic diff / lineage and their
//! Mermaid + DOT renderers (ADR-0060, W1 T9).
//!
//! Goldens live under `tests/golden/diff/` and are compared
//! byte-for-byte across reruns and store rebuilds — same posture as
//! the JSON-Schema and SHACL tests.

use convergio_db::Pool;
use convergio_ontology::{
    diff_object, lineage_object, render_diff_dot, render_diff_mermaid, render_lineage_dot,
    render_lineage_mermaid, OwnerKind, Store,
};
use serde_json::json;

async fn seeded_store() -> anyhow::Result<(tempfile::TempDir, Store)> {
    let dir = tempfile::tempdir()?;
    let url = format!("sqlite://{}?mode=rwc", dir.path().join("g.db").display());
    let pool = Pool::connect(&url).await?;
    let store = Store::new(pool);
    store.migrate().await?;
    seed(&store).await?;
    Ok((dir, store))
}

async fn seed(store: &Store) -> anyhow::Result<()> {
    // Person v1: { email }
    store
        .upsert_object("Person", 1, false, "Person", "v1.", json!({}), None)
        .await?;
    store
        .upsert_property(
            "email",
            1,
            false,
            "Email",
            "v1 email.",
            OwnerKind::Object,
            "Person",
            "string",
            true,
            json!({"maxLength": 254}),
            None,
        )
        .await?;
    // Person v2: email (modified) + age (added). v2 is non-breaking.
    store
        .upsert_object(
            "Person",
            2,
            false,
            "Person",
            "v2 with age.",
            json!({}),
            None,
        )
        .await?;
    store
        .upsert_property(
            "email",
            2,
            false,
            "Email",
            "v2 email with stricter format.",
            OwnerKind::Object,
            "Person",
            "string",
            true,
            json!({"maxLength": 254, "format": "email"}),
            None,
        )
        .await?;
    store
        .upsert_property(
            "age",
            2,
            false,
            "Age",
            "Years.",
            OwnerKind::Object,
            "Person",
            "integer",
            false,
            json!({"minimum": 0}),
            None,
        )
        .await?;
    // Person v3 marks a breaking change for lineage rendering.
    store
        .upsert_object("Person", 3, true, "Person", "v3 breaking.", json!({}), None)
        .await?;
    Ok(())
}

#[tokio::test]
async fn diff_mermaid_matches_golden() -> anyhow::Result<()> {
    let (_d, store) = seeded_store().await?;
    let d = diff_object(&store, "Person", 1, 2).await?;
    let got = render_diff_mermaid(&d);
    let want = include_str!("golden/diff/person_v1_to_v2.diff.mermaid");
    assert_eq!(got, want, "diff mermaid drift");
    Ok(())
}

#[tokio::test]
async fn diff_dot_matches_golden() -> anyhow::Result<()> {
    let (_d, store) = seeded_store().await?;
    let d = diff_object(&store, "Person", 1, 2).await?;
    let got = render_diff_dot(&d);
    let want = include_str!("golden/diff/person_v1_to_v2.diff.dot");
    assert_eq!(got, want, "diff dot drift");
    Ok(())
}

#[tokio::test]
async fn lineage_mermaid_matches_golden() -> anyhow::Result<()> {
    let (_d, store) = seeded_store().await?;
    let l = lineage_object(&store, "Person").await?;
    assert_eq!(l.nodes.len(), 3);
    let got = render_lineage_mermaid(&l);
    let want = include_str!("golden/diff/person.lineage.mermaid");
    assert_eq!(got, want, "lineage mermaid drift");
    Ok(())
}

#[tokio::test]
async fn lineage_dot_matches_golden() -> anyhow::Result<()> {
    let (_d, store) = seeded_store().await?;
    let l = lineage_object(&store, "Person").await?;
    let got = render_lineage_dot(&l);
    let want = include_str!("golden/diff/person.lineage.dot");
    assert_eq!(got, want, "lineage dot drift");
    Ok(())
}

#[tokio::test]
async fn diff_rerun_is_byte_identical() -> anyhow::Result<()> {
    let (_d, store) = seeded_store().await?;
    let a = render_diff_mermaid(&diff_object(&store, "Person", 1, 2).await?);
    let b = render_diff_mermaid(&diff_object(&store, "Person", 1, 2).await?);
    assert_eq!(a, b);
    let a = render_diff_dot(&diff_object(&store, "Person", 1, 2).await?);
    let b = render_diff_dot(&diff_object(&store, "Person", 1, 2).await?);
    assert_eq!(a, b);
    Ok(())
}

#[tokio::test]
async fn lineage_stable_across_store_rebuild() -> anyhow::Result<()> {
    let (_d1, store1) = seeded_store().await?;
    let (_d2, store2) = seeded_store().await?;
    let l1 = render_lineage_mermaid(&lineage_object(&store1, "Person").await?);
    let l2 = render_lineage_mermaid(&lineage_object(&store2, "Person").await?);
    assert_eq!(l1, l2);
    Ok(())
}

#[tokio::test]
async fn diff_self_is_noop() -> anyhow::Result<()> {
    let (_d, store) = seeded_store().await?;
    let d = diff_object(&store, "Person", 2, 2).await?;
    assert!(d.is_empty(), "v2 vs v2 should be empty: {:?}", d);
    Ok(())
}

#[tokio::test]
async fn diff_inverted_range_errors() -> anyhow::Result<()> {
    let (_d, store) = seeded_store().await?;
    let err = diff_object(&store, "Person", 2, 1).await.unwrap_err();
    assert!(matches!(
        err,
        convergio_ontology::Error::NotImplemented { .. }
    ));
    Ok(())
}

#[tokio::test]
async fn lineage_not_found_errors() -> anyhow::Result<()> {
    let (_d, store) = seeded_store().await?;
    let err = lineage_object(&store, "Nope").await.unwrap_err();
    assert!(matches!(err, convergio_ontology::Error::NotFound { .. }));
    Ok(())
}
