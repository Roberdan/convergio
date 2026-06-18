//! CSV source connector.
//!
//! Ingests CSV held in memory (string or bytes) and emits one
//! `serde_json::Value` object per data row, keyed by header (or `col_N`
//! when the source is header-less). The parser is a small, dependency-free
//! RFC 4180-style state machine (quoted fields, escaped `""`, embedded
//! delimiters and newlines inside quotes).

use super::{latest_watermark, page_records, stable_schema_hash};
use crate::connector::{Connector, DiscoverItem, DiscoverRequest, Health, PullPage, PullRequest};
use crate::error::ConnectorError;
use crate::types::{SchemaHash, Watermark};
use async_trait::async_trait;
use serde::Serialize;
use serde_json::{Map, Value};

/// Configuration for the CSV connector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CsvConfig {
    /// Stable stream/dataset identifier exposed via `discover`.
    pub stream: String,
    /// Field delimiter (typically `,`).
    pub delimiter: char,
    /// Whether the first row carries column headers.
    pub has_headers: bool,
}

impl Default for CsvConfig {
    fn default() -> Self {
        Self {
            stream: "csv".to_string(),
            delimiter: ',',
            has_headers: true,
        }
    }
}

impl CsvConfig {
    /// Validate the configuration, returning a clear error on misuse.
    pub fn validate(&self) -> Result<(), ConnectorError> {
        if self.stream.trim().is_empty() {
            return Err(ConnectorError::protocol("csv.stream must be non-empty"));
        }
        if matches!(self.delimiter, '"' | '\n' | '\r') {
            return Err(ConnectorError::protocol(
                "csv.delimiter must not be a quote or newline",
            ));
        }
        Ok(())
    }
}

/// A CSV source connector over an in-memory document.
#[derive(Debug, Clone)]
pub struct CsvConnector {
    config: CsvConfig,
    data: String,
}

impl CsvConnector {
    /// Build a connector from a validated config and an in-memory CSV string.
    pub fn new(config: CsvConfig, data: impl Into<String>) -> Result<Self, ConnectorError> {
        config.validate()?;
        Ok(Self {
            config,
            data: data.into(),
        })
    }

    /// Build a connector from raw UTF-8 bytes (e.g. a file read into memory).
    pub fn from_bytes(config: CsvConfig, bytes: &[u8]) -> Result<Self, ConnectorError> {
        let text = std::str::from_utf8(bytes)
            .map_err(|e| ConnectorError::protocol(format!("csv is not valid utf-8: {e}")))?;
        Self::new(config, text)
    }

    /// Parse the document into ordered JSON record objects.
    fn records(&self) -> Result<Vec<Value>, ConnectorError> {
        let rows = parse_rows(&self.data, self.config.delimiter)?;
        let mut iter = rows.into_iter();

        let headers: Vec<String> = if self.config.has_headers {
            match iter.next() {
                Some(h) => h,
                None => return Ok(Vec::new()),
            }
        } else {
            Vec::new()
        };

        let mut out = Vec::new();
        for row in iter {
            out.push(row_to_object(&headers, row));
        }
        Ok(out)
    }
}

/// Map a CSV row to a JSON object using `headers` (or synthetic `col_N` keys).
fn row_to_object(headers: &[String], row: Vec<String>) -> Value {
    let mut obj = Map::new();
    let width = headers.len().max(row.len());
    for i in 0..width {
        let key = headers
            .get(i)
            .filter(|h| !h.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| format!("col_{i}"));
        let val = row.get(i).cloned().unwrap_or_default();
        obj.insert(key, Value::String(val));
    }
    Value::Object(obj)
}

/// Parse CSV text into rows of string fields (RFC 4180-style).
fn parse_rows(input: &str, delim: char) -> Result<Vec<Vec<String>>, ConnectorError> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut record: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if in_quotes {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    field.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            } else {
                field.push(c);
            }
        } else if c == '"' {
            in_quotes = true;
        } else if c == delim {
            record.push(std::mem::take(&mut field));
        } else if c == '\n' {
            record.push(std::mem::take(&mut field));
            push_record(&mut rows, std::mem::take(&mut record));
        } else if c != '\r' {
            field.push(c);
        }
    }

    if in_quotes {
        return Err(ConnectorError::protocol(
            "csv has an unterminated quoted field",
        ));
    }
    if !field.is_empty() || !record.is_empty() {
        record.push(field);
        push_record(&mut rows, record);
    }
    Ok(rows)
}

/// Append a record, skipping fully blank lines.
fn push_record(rows: &mut Vec<Vec<String>>, record: Vec<String>) {
    if record.len() == 1 && record[0].is_empty() {
        return;
    }
    rows.push(record);
}

#[async_trait]
impl Connector for CsvConnector {
    type Record = Value;

    async fn discover(&self, _req: DiscoverRequest) -> Result<Vec<DiscoverItem>, ConnectorError> {
        Ok(vec![DiscoverItem {
            stream: self.config.stream.clone(),
            label: format!("CSV stream '{}'", self.config.stream),
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
        let all = self.records()?;
        page_records(&all, req.since.as_ref(), req.limit)
    }

    async fn watermark(&self) -> Result<Option<Watermark>, ConnectorError> {
        Ok(latest_watermark(self.records()?.len()))
    }

    async fn schema_hash(&self) -> Result<SchemaHash, ConnectorError> {
        stable_schema_hash(&self.config)
    }

    async fn health(&self) -> Result<Health, ConnectorError> {
        match self.records() {
            Ok(_) => Ok(Health::Healthy),
            Err(_) => Ok(Health::Unhealthy),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CSV: &str = "id,name\n1,Ada\n2,\"Grace, Hopper\"\n3,\"line\nbreak\"\n";

    fn conn() -> CsvConnector {
        CsvConnector::new(CsvConfig::default(), CSV).expect("connector")
    }

    #[tokio::test]
    async fn pulls_rows_in_order_with_quoting() {
        let page = conn().pull(PullRequest::default()).await.expect("pull");
        assert_eq!(page.records.len(), 3);
        assert_eq!(page.records[0], serde_json::json!({"id":"1","name":"Ada"}));
        assert_eq!(
            page.records[1],
            serde_json::json!({"id":"2","name":"Grace, Hopper"})
        );
        assert_eq!(
            page.records[2],
            serde_json::json!({"id":"3","name":"line\nbreak"})
        );
        assert!(!page.has_more);
    }

    #[tokio::test]
    async fn paging_resumes_from_watermark() {
        let c = conn();
        let first = c
            .pull(PullRequest {
                limit: 1,
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
        assert_eq!(rest.records.len(), 2);
        assert_eq!(rest.records[0]["id"], serde_json::json!("2"));
    }

    #[tokio::test]
    async fn headerless_uses_synthetic_keys() {
        let cfg = CsvConfig {
            has_headers: false,
            ..Default::default()
        };
        let c = CsvConnector::new(cfg, "a,b\nc,d\n").expect("conn");
        let page = c.pull(PullRequest::default()).await.expect("pull");
        assert_eq!(page.records.len(), 2);
        assert_eq!(
            page.records[0],
            serde_json::json!({"col_0":"a","col_1":"b"})
        );
    }

    #[tokio::test]
    async fn watermark_and_schema_hash_are_stable() {
        let c = conn();
        let wm = c.watermark().await.expect("wm").expect("some");
        assert_eq!(super::super::index_from_watermark(&wm).expect("idx"), 2);
        let h1 = c.schema_hash().await.expect("h1");
        let h2 = c.schema_hash().await.expect("h2");
        assert_eq!(h1, h2);
        assert_eq!(h1.as_hex().len(), 64);
    }

    #[tokio::test]
    async fn unterminated_quote_is_an_error() {
        let c = CsvConnector::new(CsvConfig::default(), "id\n\"oops\n").expect("conn");
        let err = c.pull(PullRequest::default()).await.unwrap_err();
        assert!(err.to_string().contains("unterminated"));
        assert_eq!(c.health().await.expect("health"), Health::Unhealthy);
    }

    #[test]
    fn rejects_invalid_delimiter() {
        let cfg = CsvConfig {
            delimiter: '"',
            ..Default::default()
        };
        assert!(CsvConnector::new(cfg, "a\n").is_err());
    }
}
