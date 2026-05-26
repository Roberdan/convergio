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
}
