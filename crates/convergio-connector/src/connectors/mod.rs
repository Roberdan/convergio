//! Reference source connectors built on the [`Connector`](crate::Connector) trait.
//!
//! This module ships two production-quality, fully-wired reference
//! connectors so the SDK is no longer trait-only:
//!
//! - [`CsvConnector`] — ingests CSV from an in-memory string/bytes reader.
//! - [`HttpJsonConnector`] — ingests JSON records via an injectable
//!   [`JsonFetcher`], so it is deterministic and unit-testable offline
//!   (no live HTTP server required).
//!
//! Both connectors emit records as `serde_json::Value` objects (one per
//! row/element), preserve **source order** for determinism, and page via a
//! numeric [`Watermark`] that is exclusive on `since` per the trait contract.

mod csv;
mod http_json;

pub use csv::{CsvConfig, CsvConnector};
pub use http_json::{HttpJsonConfig, HttpJsonConnector, JsonFetcher, StaticJsonFetcher};

use crate::connector::PullPage;
use crate::error::ConnectorError;
use crate::types::{SchemaHash, Watermark};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Zero-pad width for watermark indices (keeps them lexicographically sortable
/// for human inspection; parsing ignores leading zeros).
const WM_WIDTH: usize = 12;

/// Build a [`Watermark`] from a zero-based record index.
pub(crate) fn watermark_from_index(i: usize) -> Watermark {
    Watermark::new(format!("{i:0WM_WIDTH$}"))
}

/// Parse a [`Watermark`] back into a zero-based record index.
pub(crate) fn index_from_watermark(w: &Watermark) -> Result<usize, ConnectorError> {
    w.0.trim()
        .parse::<usize>()
        .map_err(|_| ConnectorError::protocol(format!("invalid watermark cursor: {:?}", w.0)))
}

/// Page an ordered slice of records by `since` (exclusive) and `limit`.
///
/// Ordering is the caller-provided slice order, which both reference
/// connectors derive from source order, guaranteeing deterministic output.
pub(crate) fn page_records(
    all: &[Value],
    since: Option<&Watermark>,
    limit: u32,
) -> Result<PullPage<Value>, ConnectorError> {
    let start = match since {
        Some(w) => index_from_watermark(w)?.saturating_add(1),
        None => 0,
    }
    .min(all.len());

    let window = if limit == 0 {
        all.len()
    } else {
        limit as usize
    };
    let end = start.saturating_add(window).min(all.len());

    let records: Vec<Value> = all[start..end].to_vec();
    let has_more = end < all.len();
    let next_watermark = if end > start {
        Some(watermark_from_index(end - 1))
    } else {
        None
    };

    Ok(PullPage {
        records,
        next_watermark,
        has_more,
    })
}

/// Report the newest available watermark for an ordered record set.
pub(crate) fn latest_watermark(len: usize) -> Option<Watermark> {
    if len == 0 {
        None
    } else {
        Some(watermark_from_index(len - 1))
    }
}

/// Compute a stable schema hash over a connector's canonical config form.
pub(crate) fn stable_schema_hash(parts: &impl Serialize) -> Result<SchemaHash, ConnectorError> {
    let bytes = crate::canonical_json::to_canonical_bytes(parts)?;
    let digest = Sha256::digest(bytes);
    Ok(SchemaHash::new_hex(hex::encode(digest)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(n: usize) -> Vec<Value> {
        (0..n).map(|i| serde_json::json!({ "i": i })).collect()
    }

    #[test]
    fn watermark_roundtrip() {
        let w = watermark_from_index(7);
        assert_eq!(index_from_watermark(&w).expect("idx"), 7);
    }

    #[test]
    fn paging_is_exclusive_on_since() {
        let all = rows(5);
        let page = page_records(&all, None, 2).expect("page");
        assert_eq!(page.records.len(), 2);
        assert!(page.has_more);
        let wm = page.next_watermark.clone().expect("wm");
        assert_eq!(index_from_watermark(&wm).expect("idx"), 1);

        let page2 = page_records(&all, Some(&wm), 100).expect("page2");
        assert_eq!(page2.records.len(), 3);
        assert!(!page2.has_more);
        assert_eq!(page2.records[0], serde_json::json!({ "i": 2 }));
    }

    #[test]
    fn paging_past_end_is_empty() {
        let all = rows(2);
        let wm = watermark_from_index(10);
        let page = page_records(&all, Some(&wm), 0).expect("page");
        assert!(page.records.is_empty());
        assert!(!page.has_more);
        assert!(page.next_watermark.is_none());
    }

    #[test]
    fn rejects_bad_watermark() {
        let all = rows(1);
        let bad = Watermark::new("not-a-number");
        let err = page_records(&all, Some(&bad), 1).unwrap_err();
        assert!(err.to_string().contains("invalid watermark"));
    }
}
