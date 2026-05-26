//! Integration test: the sandboxed process connector can roundtrip the protocol.
#![allow(missing_docs)]

use convergio_connector::{Connector, DiscoverRequest, ProcessConnector, ProcessConnectorSpec};
use std::path::PathBuf;
use std::time::Duration;

#[tokio::test]
async fn process_connector_roundtrips_protocol() {
    let exe = PathBuf::from(env!("CARGO_BIN_EXE_connector-shim"));
    let spec = ProcessConnectorSpec {
        command: exe,
        args: Vec::new(),
        timeout: Duration::from_secs(2),
        max_calls_per_sec: Some(100.0),
        ..Default::default()
    };

    let c = ProcessConnector::spawn(spec).await.expect("spawn");

    let h = c.health().await.expect("health");
    assert_eq!(serde_json::to_string(&h).unwrap(), "\"healthy\"");

    let hash1 = c.schema_hash().await.expect("schema_hash");
    let hash2 = c.schema_hash().await.expect("schema_hash2");
    assert_eq!(hash1, hash2);

    let items = c
        .discover(DiscoverRequest::default())
        .await
        .expect("discover");
    assert_eq!(items.len(), 1);

    let wm = c.watermark().await.expect("watermark");
    assert_eq!(wm.unwrap().0, "w0");

    let page = c
        .pull(convergio_connector::PullRequest {
            stream: Some("people".to_string()),
            since: None,
            limit: 10,
        })
        .await
        .expect("pull");
    assert_eq!(page.records.len(), 1);
}

#[tokio::test]
async fn process_connector_does_not_inherit_ambient_env() {
    std::env::set_var("CONVERGIO_TEST_AMBIENT_SECRET", "1");
    let exe = PathBuf::from(env!("CARGO_BIN_EXE_connector-shim"));
    let spec = ProcessConnectorSpec {
        command: exe,
        args: Vec::new(),
        timeout: Duration::from_secs(2),
        ..Default::default()
    };

    let c = ProcessConnector::spawn(spec).await.expect("spawn");

    // Clean up the parent process env immediately; if the runner incorrectly
    // inherited it at spawn time, the child would still see it.
    std::env::remove_var("CONVERGIO_TEST_AMBIENT_SECRET");

    c.health()
        .await
        .expect("health should succeed without leaked env");
}

#[tokio::test]
async fn process_connector_passes_explicit_env_only() {
    std::env::remove_var("CONVERGIO_TEST_INJECTED");
    let exe = PathBuf::from(env!("CARGO_BIN_EXE_connector-shim"));
    let mut spec = ProcessConnectorSpec {
        command: exe,
        args: Vec::new(),
        timeout: Duration::from_secs(2),
        ..Default::default()
    };
    spec.env
        .insert("CONVERGIO_TEST_INJECTED".to_string(), "1".to_string());

    let c = ProcessConnector::spawn(spec).await.expect("spawn");
    let h = c.schema_hash().await.expect("schema_hash");
    assert_eq!(h.as_hex(), &"1".repeat(64));
}

#[tokio::test]
async fn process_connector_retries_retryable_failures() {
    let exe = PathBuf::from(env!("CARGO_BIN_EXE_connector-shim"));
    let mut spec = ProcessConnectorSpec {
        command: exe,
        args: Vec::new(),
        timeout: Duration::from_secs(2),
        max_retries: 3,
        ..Default::default()
    };
    spec.env
        .insert("CONVERGIO_TEST_PULL_FAILS".to_string(), "2".to_string());

    let c = ProcessConnector::spawn(spec).await.expect("spawn");
    let page = c
        .pull(convergio_connector::PullRequest {
            stream: Some("people".to_string()),
            since: None,
            limit: 10,
        })
        .await
        .expect("pull should succeed after retries");
    assert_eq!(page.records.len(), 1);
}
