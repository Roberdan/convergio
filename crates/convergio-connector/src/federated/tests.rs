//! Unit tests for the federated query path, built on the reference connectors.

use super::*;
use crate::connectors::StaticJsonFetcher;
use crate::connectors::{CsvConfig, CsvConnector, HttpJsonConfig, HttpJsonConnector};
use std::sync::Arc;

const CSV: &str = "id,name\n1,Ada\n2,Grace\n3,Edsger\n";

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
async fn merges_two_sources_in_stable_order_with_provenance() {
    let query = FederatedQuery::new([
        csv_source("csv-1"),
        http_source("http-1", r#"[{"id":"9","name":"Linus"}]"#),
    ])
    .with_per_source_limit(10)
    .with_result_cap(10);

    let exec = FederatedExecutor::new(FederationPolicy::unrestricted());
    let result = exec.execute(&query).await.expect("execute");

    assert!(!result.truncated);
    assert_eq!(result.records.len(), 4);
    // CSV source (order 0) comes first, in source order.
    assert_eq!(result.records[0].source_id, ConnectorId::new("csv-1"));
    assert_eq!(result.records[0].source_order, 0);
    assert_eq!(result.records[0].record["name"], serde_json::json!("Ada"));
    assert_eq!(
        result.records[2].record["name"],
        serde_json::json!("Edsger")
    );
    // HTTP source (order 1) comes after all CSV records.
    assert_eq!(result.records[3].source_id, ConnectorId::new("http-1"));
    assert_eq!(result.records[3].source_order, 1);
    assert_eq!(result.records[3].record["name"], serde_json::json!("Linus"));

    assert_eq!(result.counts.len(), 2);
    assert_eq!(result.counts[0].count, 3);
    assert_eq!(result.counts[1].count, 1);
}

#[tokio::test]
async fn refuses_over_cap_query_without_executing() {
    // A source whose fetcher would error if pulled, proving the gate refuses
    // before any connector runs.
    let exploding = http_source("http-bad", "not json at all");
    let query = FederatedQuery::new([csv_source("csv-1"), exploding])
        .with_per_source_limit(5)
        .with_result_cap(5);

    let policy = FederationPolicy::unrestricted().with_max_sources(1);
    let exec = FederatedExecutor::new(policy);
    let err = exec.execute(&query).await.unwrap_err();
    match err {
        ConnectorError::FederationRefused { reason } => {
            assert!(reason.contains("too many sources"), "reason: {reason}");
        }
        other => panic!("expected FederationRefused, got {other:?}"),
    }
}

#[tokio::test]
async fn refuses_disallowed_source_kind() {
    let query = FederatedQuery::new([http_source("http-1", "[]")])
        .with_per_source_limit(5)
        .with_result_cap(5);
    let policy = FederationPolicy::unrestricted().with_allowed_kinds(["csv"]);
    let err = FederatedExecutor::new(policy)
        .execute(&query)
        .await
        .unwrap_err();
    assert!(matches!(err, ConnectorError::FederationRefused { .. }));
}

#[tokio::test]
async fn honours_per_source_limit() {
    let query = FederatedQuery::new([csv_source("csv-1")])
        .with_per_source_limit(2)
        .with_result_cap(100);
    let exec = FederatedExecutor::new(FederationPolicy::unrestricted());
    let result = exec.execute(&query).await.expect("execute");
    assert_eq!(result.records.len(), 2);
    assert_eq!(result.records[0].record["name"], serde_json::json!("Ada"));
    assert_eq!(result.records[1].record["name"], serde_json::json!("Grace"));
    assert!(!result.truncated);
}

#[tokio::test]
async fn result_cap_truncates_deterministically() {
    let query = FederatedQuery::new([
        csv_source("csv-1"),
        http_source("http-1", r#"[{"id":"9","name":"Linus"}]"#),
    ])
    .with_per_source_limit(10)
    .with_result_cap(2);
    let exec = FederatedExecutor::new(FederationPolicy::unrestricted());
    let result = exec.execute(&query).await.expect("execute");
    assert!(result.truncated);
    assert_eq!(result.records.len(), 2);
    // First two of the stable order are the CSV source's first two rows.
    assert_eq!(result.records[0].record["name"], serde_json::json!("Ada"));
    assert_eq!(result.records[1].record["name"], serde_json::json!("Grace"));
    assert_eq!(result.counts[0].count, 2);
    assert_eq!(result.counts[1].count, 0);
}

#[tokio::test]
async fn projection_selects_fields() {
    let query = FederatedQuery::new([csv_source("csv-1")])
        .with_per_source_limit(10)
        .with_result_cap(10)
        .with_projection(Projection::new(["name"]));
    let exec = FederatedExecutor::new(FederationPolicy::unrestricted());
    let result = exec.execute(&query).await.expect("execute");
    assert_eq!(result.records[0].record, serde_json::json!({"name": "Ada"}));
}

#[test]
fn policy_refuses_unbounded_cap_under_total_max() {
    let query = FederatedQuery::new([csv_source("csv-1")]).with_per_source_limit(5);
    let policy = FederationPolicy::unrestricted().with_max_total_records(10);
    let err = policy.evaluate(&query).unwrap_err();
    assert!(matches!(err, ConnectorError::FederationRefused { .. }));
}
