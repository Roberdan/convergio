//! HTTP-JSON source connector.
//!
//! Ingests JSON records and emits one `serde_json::Value` object per element.
//! Transport is abstracted behind the [`JsonFetcher`] trait so the connector
//! is **deterministic and offline-testable**: tests inject a
//! [`StaticJsonFetcher`] holding in-memory bytes instead of hitting a live
//! HTTP server. A real network fetcher can be supplied by an outer crate that
//! owns an HTTP client, without changing this connector.

use super::{latest_watermark, page_records, stable_schema_hash};
use crate::connector::{Connector, DiscoverItem, DiscoverRequest, Health, PullPage, PullRequest};
use crate::error::ConnectorError;
use crate::types::{SchemaHash, Watermark};
use async_trait::async_trait;
use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;

/// Pluggable, dependency-injected source of raw JSON bytes.
///
/// Implementors fetch the payload however they like (in-memory, file, or a
/// real HTTP client owned by another crate). Keeping this a trait is what lets
/// the connector be unit-tested without a network.
#[async_trait]
pub trait JsonFetcher: Send + Sync {
    /// Fetch the raw JSON document as bytes.
    async fn fetch(&self) -> Result<Vec<u8>, ConnectorError>;
}

/// In-memory [`JsonFetcher`] used for deterministic, offline tests.
#[derive(Debug, Clone)]
pub struct StaticJsonFetcher {
    bytes: Vec<u8>,
}

impl StaticJsonFetcher {
    /// Build a fetcher from raw bytes.
    pub fn new(bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            bytes: bytes.into(),
        }
    }

    /// Build a fetcher from an already-parsed JSON value.
    pub fn from_value(value: &Value) -> Result<Self, ConnectorError> {
        Ok(Self::new(serde_json::to_vec(value)?))
    }
}

#[async_trait]
impl JsonFetcher for StaticJsonFetcher {
    async fn fetch(&self) -> Result<Vec<u8>, ConnectorError> {
        Ok(self.bytes.clone())
    }
}

/// Configuration for the HTTP-JSON connector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HttpJsonConfig {
    /// Stable stream/dataset identifier exposed via `discover`.
    pub stream: String,
    /// Optional RFC 6901 JSON Pointer to the array of records. When `None`,
    /// the document root itself must be an array.
    pub records_pointer: Option<String>,
}

impl Default for HttpJsonConfig {
    fn default() -> Self {
        Self {
            stream: "http-json".to_string(),
            records_pointer: None,
        }
    }
}

impl HttpJsonConfig {
    /// Validate the configuration, returning a clear error on misuse.
    pub fn validate(&self) -> Result<(), ConnectorError> {
        if self.stream.trim().is_empty() {
            return Err(ConnectorError::protocol(
                "http_json.stream must be non-empty",
            ));
        }
        if let Some(ptr) = &self.records_pointer {
            if !ptr.starts_with('/') {
                return Err(ConnectorError::protocol(
                    "http_json.records_pointer must be a JSON Pointer starting with '/'",
                ));
            }
        }
        Ok(())
    }
}

/// An HTTP-JSON source connector over an injectable [`JsonFetcher`].
#[derive(Clone)]
pub struct HttpJsonConnector {
    config: HttpJsonConfig,
    fetcher: Arc<dyn JsonFetcher>,
}

impl HttpJsonConnector {
    /// Build a connector from a validated config and an injected fetcher.
    pub fn new(
        config: HttpJsonConfig,
        fetcher: Arc<dyn JsonFetcher>,
    ) -> Result<Self, ConnectorError> {
        config.validate()?;
        Ok(Self { config, fetcher })
    }

    /// Fetch and extract the ordered array of record objects.
    async fn records(&self) -> Result<Vec<Value>, ConnectorError> {
        let bytes = self.fetcher.fetch().await?;
        let doc: Value = serde_json::from_slice(&bytes)?;
        let array = match &self.config.records_pointer {
            Some(ptr) => doc.pointer(ptr).ok_or_else(|| {
                ConnectorError::protocol(format!("no JSON value at pointer {ptr}"))
            })?,
            None => &doc,
        };
        let items = array
            .as_array()
            .ok_or_else(|| ConnectorError::protocol("http_json records target is not an array"))?;

        let mut out = Vec::with_capacity(items.len());
        for (i, item) in items.iter().enumerate() {
            if !item.is_object() {
                return Err(ConnectorError::protocol(format!(
                    "http_json record at index {i} is not an object"
                )));
            }
            out.push(item.clone());
        }
        Ok(out)
    }
}

#[async_trait]
impl Connector for HttpJsonConnector {
    type Record = Value;

    async fn discover(&self, _req: DiscoverRequest) -> Result<Vec<DiscoverItem>, ConnectorError> {
        Ok(vec![DiscoverItem {
            stream: self.config.stream.clone(),
            label: format!("HTTP-JSON stream '{}'", self.config.stream),
        }])
    }

    async fn pull(&self, req: PullRequest) -> Result<PullPage<Self::Record>, ConnectorError> {
        if let Some(stream) = &req.stream {
            if stream != &self.config.stream {
                return Err(ConnectorError::protocol(format!(
                    "unknown stream: {stream}"
                )));
            }
        }
        let all = self.records().await?;
        page_records(&all, req.since.as_ref(), req.limit)
    }

    async fn watermark(&self) -> Result<Option<Watermark>, ConnectorError> {
        Ok(latest_watermark(self.records().await?.len()))
    }

    async fn schema_hash(&self) -> Result<SchemaHash, ConnectorError> {
        stable_schema_hash(&self.config)
    }

    async fn health(&self) -> Result<Health, ConnectorError> {
        match self.records().await {
            Ok(_) => Ok(Health::Healthy),
            Err(_) => Ok(Health::Unhealthy),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn array_conn(json: &str) -> HttpJsonConnector {
        let fetcher = Arc::new(StaticJsonFetcher::new(json.as_bytes().to_vec()));
        HttpJsonConnector::new(HttpJsonConfig::default(), fetcher).expect("conn")
    }

    #[tokio::test]
    async fn pulls_root_array_in_order() {
        let c = array_conn(r#"[{"id":1},{"id":2},{"id":3}]"#);
        let page = c.pull(PullRequest::default()).await.expect("pull");
        assert_eq!(page.records.len(), 3);
        assert_eq!(page.records[0], serde_json::json!({"id":1}));
        assert_eq!(page.records[2], serde_json::json!({"id":3}));
        assert!(!page.has_more);
    }

    #[tokio::test]
    async fn extracts_via_json_pointer() {
        let fetcher = Arc::new(StaticJsonFetcher::new(
            r#"{"data":{"items":[{"k":"a"},{"k":"b"}]}}"#.as_bytes().to_vec(),
        ));
        let cfg = HttpJsonConfig {
            records_pointer: Some("/data/items".to_string()),
            ..Default::default()
        };
        let c = HttpJsonConnector::new(cfg, fetcher).expect("conn");
        let page = c.pull(PullRequest::default()).await.expect("pull");
        assert_eq!(page.records.len(), 2);
        assert_eq!(page.records[1], serde_json::json!({"k":"b"}));
    }

    #[tokio::test]
    async fn paging_resumes_from_watermark() {
        let c = array_conn(r#"[{"id":1},{"id":2},{"id":3}]"#);
        let first = c
            .pull(PullRequest {
                limit: 2,
                ..Default::default()
            })
            .await
            .expect("first");
        assert!(first.has_more);
        let wm = first.next_watermark.clone().expect("wm");
        let rest = c
            .pull(PullRequest {
                since: Some(wm),
                ..Default::default()
            })
            .await
            .expect("rest");
        assert_eq!(rest.records, vec![serde_json::json!({"id":3})]);
        assert!(!rest.has_more);
    }

    #[tokio::test]
    async fn watermark_and_schema_hash_are_stable() {
        let c = array_conn(r#"[{"id":1},{"id":2}]"#);
        let wm = c.watermark().await.expect("wm").expect("some");
        assert_eq!(super::super::index_from_watermark(&wm).expect("idx"), 1);
        let h1 = c.schema_hash().await.expect("h1");
        let h2 = c.schema_hash().await.expect("h2");
        assert_eq!(h1, h2);
        assert_eq!(h1.as_hex().len(), 64);
    }

    #[tokio::test]
    async fn non_array_payload_is_an_error() {
        let c = array_conn(r#"{"not":"an array"}"#);
        let err = c.pull(PullRequest::default()).await.unwrap_err();
        assert!(err.to_string().contains("not an array"));
        assert_eq!(c.health().await.expect("health"), Health::Unhealthy);
    }

    #[tokio::test]
    async fn non_object_element_is_an_error() {
        let c = array_conn(r#"[{"id":1}, 42]"#);
        let err = c.pull(PullRequest::default()).await.unwrap_err();
        assert!(err.to_string().contains("not an object"));
    }

    #[test]
    fn rejects_invalid_pointer() {
        let fetcher = Arc::new(StaticJsonFetcher::new(b"[]".to_vec()));
        let cfg = HttpJsonConfig {
            records_pointer: Some("data/items".to_string()),
            ..Default::default()
        };
        assert!(HttpJsonConnector::new(cfg, fetcher).is_err());
    }
}
