//! Integration tests for the reference source connectors (offline).
//!
//! These drive the connectors purely through the crate's public API to prove
//! they are fully wired and satisfy the shared connector contract.

use convergio_connector::{
    assert_basic_connector_contract, Connector, CsvConfig, CsvConnector, HttpJsonConfig,
    HttpJsonConnector, PullRequest, StaticJsonFetcher,
};
use std::sync::Arc;

#[tokio::test]
async fn csv_connector_satisfies_basic_contract() {
    let csv = "id,name\n1,Ada\n2,Grace\n";
    let conn = CsvConnector::new(CsvConfig::default(), csv).expect("connector");
    assert_basic_connector_contract(&conn)
        .await
        .expect("contract");

    let page = conn.pull(PullRequest::default()).await.expect("pull");
    assert_eq!(page.records.len(), 2);
    assert_eq!(page.records[0]["name"], serde_json::json!("Ada"));
}

#[tokio::test]
async fn http_json_connector_satisfies_basic_contract() {
    let fetcher = Arc::new(StaticJsonFetcher::new(br#"[{"id":1},{"id":2}]"#.to_vec()));
    let conn = HttpJsonConnector::new(HttpJsonConfig::default(), fetcher).expect("connector");
    assert_basic_connector_contract(&conn)
        .await
        .expect("contract");

    let discovered = conn.discover(Default::default()).await.expect("discover");
    assert_eq!(discovered.len(), 1);
    assert_eq!(discovered[0].stream, "http-json");
}
