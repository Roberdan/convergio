//! Helper to (re)generate the goldens under `tests/golden/diff/`.
//! Run with `cargo run -p convergio-ontology --example gen_diff_golden`.

use convergio_db::Pool;
use convergio_ontology::{
    diff_object, lineage_object, render_diff_dot, render_diff_mermaid, render_lineage_dot,
    render_lineage_mermaid, OwnerKind, Store,
};
use serde_json::json;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let url = format!("sqlite://{}?mode=rwc", dir.path().join("g.db").display());
    let pool = Pool::connect(&url).await?;
    let store = Store::new(pool);
    store.migrate().await?;
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
    store
        .upsert_object("Person", 3, true, "Person", "v3 breaking.", json!({}), None)
        .await?;

    let manifest: PathBuf = env!("CARGO_MANIFEST_DIR").into();
    let out = manifest.join("tests/golden/diff");

    let d = diff_object(&store, "Person", 1, 2).await?;
    std::fs::write(
        out.join("person_v1_to_v2.diff.mermaid"),
        render_diff_mermaid(&d),
    )?;
    std::fs::write(out.join("person_v1_to_v2.diff.dot"), render_diff_dot(&d))?;
    let l = lineage_object(&store, "Person").await?;
    std::fs::write(
        out.join("person.lineage.mermaid"),
        render_lineage_mermaid(&l),
    )?;
    std::fs::write(out.join("person.lineage.dot"), render_lineage_dot(&l))?;
    println!("ok");
    Ok(())
}
