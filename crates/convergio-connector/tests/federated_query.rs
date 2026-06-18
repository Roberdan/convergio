//! Integration tests for the federated query path + refusal gate (offline).
//!
//! These drive the federation layer purely through the crate's public API,
//! fanning out across the CSV and HTTP-JSON reference connectors with their
//! in-memory / injected data — no network involved.

use convergio_connector::{
    ConnectorError, ConnectorId, CsvConfig, CsvConnector, FederatedExecutor, FederatedQuery,
    FederatedSource, FederationPolicy, HttpJsonConfig, HttpJsonConnector, Projection,
    StaticJsonFetcher,
};
use std::sync::Arc;

const CSV: &str = "id,name\n1,Ada\n2,Grace\n3,Edsger\n";
const HTTP: &str = r#"[{"id":"9","name":"Linus"},{"id":"10","name":"Ken"}]"#;

fn csv_source(id: &str) -> FederatedSource {
    let conn = CsvConnector::new(CsvConfig::default(), CSV).expect("csv connector");
    FederatedSource::new(id, "CSV people", "csv", conn)
}

fn http_source(id: &str, json: &str) -> FederatedSource {
    let fetcher = Arc::new(StaticJsonFetcher::new(json.as_bytes().to_vec()));
    let conn = HttpJsonConnector::new(HttpJsonConfig::default(), fetcher).expect("http connector");
    FederatedSource::new(id, "HTTP people", "http_json", conn)
}

#[tokio::test]
async fn happy_path_merges_two_sources_in_stable_order_with_source_tags() {
    let query = FederatedQuery::new([csv_source("csv-1"), http_source("http-1", HTTP)])
        .with_per_source_limit(10)
        .with_result_cap(10);
    let exec = FederatedExecutor::new(FederationPolicy::unrestricted());
    let result = exec.execute(&query).await.expect("execute");

    assert!(!result.truncated);
    let names: Vec<_> = result
        .records
        .iter()
        .map(|r| r.record["name"].clone())
        .collect();
    assert_eq!(
        names,
        vec![
            serde_json::json!("Ada"),
            serde_json::json!("Grace"),
            serde_json::json!("Edsger"),
            serde_json::json!("Linus"),
            serde_json::json!("Ken"),
        ]
    );
    // Provenance: CSV rows (order 0) precede HTTP rows (order 1).
    assert_eq!(result.records[0].source_id, ConnectorId::new("csv-1"));
    assert_eq!(result.records[0].source_order, 0);
    assert_eq!(result.records[4].source_id, ConnectorId::new("http-1"));
    assert_eq!(result.records[4].source_order, 1);
    assert_eq!(result.records[4].source_name, "HTTP people");
}

#[tokio::test]
async fn gate_refuses_over_cap_query_without_executing() {
    // The HTTP source holds invalid JSON: if it were ever pulled the run would
    // fail with a protocol error, not a refusal. Getting a refusal proves the
    // gate ran first and no connector was contacted.
    let query = FederatedQuery::new([csv_source("csv-1"), http_source("bad", "{not json}")])
        .with_per_source_limit(5)
        .with_result_cap(5);
    let policy = FederationPolicy::unrestricted().with_max_sources(1);
    let err = FederatedExecutor::new(policy)
        .execute(&query)
        .await
        .unwrap_err();
    match err {
        ConnectorError::FederationRefused { reason } => {
            assert!(reason.contains("too many sources"), "reason: {reason}");
        }
        other => panic!("expected FederationRefused, got {other:?}"),
    }
}

#[tokio::test]
async fn gate_refuses_disallowed_source_id() {
    let query = FederatedQuery::new([csv_source("csv-1")])
        .with_per_source_limit(5)
        .with_result_cap(5);
    let policy = FederationPolicy::unrestricted().with_allowed_ids(["other"]);
    let err = FederatedExecutor::new(policy)
        .execute(&query)
        .await
        .unwrap_err();
    assert!(matches!(err, ConnectorError::FederationRefused { .. }));
}

#[tokio::test]
async fn per_source_limit_is_honoured() {
    let query = FederatedQuery::new([csv_source("csv-1"), http_source("http-1", HTTP)])
        .with_per_source_limit(1)
        .with_result_cap(100);
    let result = FederatedExecutor::new(FederationPolicy::unrestricted())
        .execute(&query)
        .await
        .expect("execute");
    assert_eq!(result.records.len(), 2);
    assert_eq!(result.counts[0].count, 1);
    assert_eq!(result.counts[1].count, 1);
    assert_eq!(result.records[0].record["name"], serde_json::json!("Ada"));
    assert_eq!(result.records[1].record["name"], serde_json::json!("Linus"));
}

#[tokio::test]
async fn result_cap_truncates_deterministically_within_policy() {
    let query = FederatedQuery::new([csv_source("csv-1"), http_source("http-1", HTTP)])
        .with_per_source_limit(10)
        .with_result_cap(4)
        .with_projection(Projection::new(["name"]));
    let policy = FederationPolicy::unrestricted()
        .with_max_sources(2)
        .with_max_total_records(10)
        .with_max_per_source_limit(10);
    let result = FederatedExecutor::new(policy)
        .execute(&query)
        .await
        .expect("execute");
    assert!(result.truncated);
    assert_eq!(result.records.len(), 4);
    let names: Vec<_> = result
        .records
        .iter()
        .map(|r| r.record["name"].clone())
        .collect();
    assert_eq!(
        names,
        vec![
            serde_json::json!("Ada"),
            serde_json::json!("Grace"),
            serde_json::json!("Edsger"),
            serde_json::json!("Linus"),
        ]
    );
    // Projection kept only the "name" field.
    assert_eq!(result.records[0].record, serde_json::json!({"name": "Ada"}));
}
