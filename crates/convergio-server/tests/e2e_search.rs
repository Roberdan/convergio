//! End-to-end test for `/api/search`.

mod common;

use common::boot;
use convergio_durability::{NewPlan, NewTask};
use convergio_embed::embedder::testing::DeterministicTestEmbedder;
use convergio_embed::{EmbedStore, Embedder, SourceText};
use convergio_graph::{Node, NodeKind, Store as GraphStore};
use serde_json::Value;
use std::sync::Arc;

#[tokio::test]
async fn search_fuses_structured_semantic_operational() {
    let (base, pool, _dir) = boot().await;

    // Ensure graph tables exist for graph-backed search.
    let graph = GraphStore::new(pool.clone());
    graph.migrate().await.expect("graph migrate");

    let durability = convergio_durability::Durability::new(pool.clone());

    let plan = durability
        .create_plan(NewPlan {
            title: "Unified search surface".into(),
            description: Some("Fuse sources".into()),
            project: Some("ontology".into()),
        })
        .await
        .expect("create plan");

    let _task = durability
        .create_task(
            &plan.id,
            NewTask {
                wave: 1,
                sequence: 1,
                title: "Implement /api/search".into(),
                description: Some("typed results".into()),
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .expect("create task");

    // Seed one graph node that should match the query.
    graph
        .upsert_node(&Node {
            id: "item:search::handler".into(),
            kind: NodeKind::Item,
            name: "search".into(),
            file_path: Some("crates/convergio-server/src/routes/search/mod.rs".into()),
            crate_name: "convergio-server".into(),
            repo: "convergio".into(),
            item_kind: Some("fn"),
            span: None,
        })
        .await
        .expect("upsert node");

    // Seed embeddings so semantic search yields a hit.
    let embed = Arc::new(EmbedStore::new(pool.clone()));
    let e = DeterministicTestEmbedder::new(8);
    let node_id = "docs/search-design.md";
    let v = e.embed(node_id).expect("embed");
    let h = SourceText::new(node_id).source_hash;
    embed
        .upsert("convergio", node_id, e.model_id(), &v, &h)
        .await
        .expect("upsert embed");

    let client = common::client();
    let body: Value = client
        .get(format!("{base}/api/search"))
        .query(&[("q", "search")])
        .send()
        .await
        .expect("send")
        .json()
        .await
        .expect("json");

    assert_eq!(body["query"], "search");
    let results = body["results"].as_array().expect("results array");
    assert!(!results.is_empty());

    // Expect at least one of each: plan, graph_node, and doc.
    assert!(results.iter().any(|r| r["type"] == "plan"));
    assert!(results.iter().any(|r| r["type"] == "graph_node"));
    assert!(results.iter().any(|r| r["type"] == "doc"));

    // Href must be routable and percent-encode `id` as a single segment.
    let doc = results
        .iter()
        .find(|r| r["type"] == "doc")
        .expect("doc result");
    assert!(doc["href"].as_str().unwrap().starts_with("/o/doc/"));
    assert!(doc["href"].as_str().unwrap().contains("%2F"));
}
