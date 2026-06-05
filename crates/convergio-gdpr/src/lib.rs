//! GDPR data-subject-rights handlers for Convergio (ADR-0076).
//!
//! The crate stays leaf-only: callers provide subject-scoped records,
//! and this crate returns structured Article 15/17/20 responses without
//! depending on SQLite, HTTP, or the audit layer.

#![deny(missing_docs)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;

/// Stable opaque identifier of a data subject within Convergio.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DataSubjectId(pub String);

/// The GDPR rights represented in Convergio's request contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GdprRight {
    /// Art. 15 — right of access.
    Access,
    /// Art. 16 — right to rectification.
    Rectification,
    /// Art. 17 — right to erasure.
    Erasure,
    /// Art. 18 — right to restriction of processing.
    Restriction,
    /// Art. 20 — right to data portability.
    Portability,
    /// Art. 21 — right to object.
    Objection,
    /// Art. 22 — automated individual decision-making safeguards.
    AutomatedDecisionSafeguards,
}

/// A request received from, or on behalf of, a data subject.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSubjectRequest {
    /// The subject identifier.
    pub subject: DataSubjectId,
    /// The specific right being invoked.
    pub right: GdprRight,
    /// When the controller received the request.
    pub received_at: DateTime<Utc>,
    /// Optional non-sensitive operator note or scope hint.
    pub note: Option<String>,
}

/// Subject-scoped record supplied by a caller for GDPR processing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DataSubjectRecord {
    /// Stable record identifier.
    pub record_id: String,
    /// Logical namespace, such as `ontology.object` or `evidence`.
    pub namespace: String,
    /// Record payload to include in access and portability exports.
    pub payload: Value,
    /// Whether this record is eligible for Article 20 export.
    #[serde(default = "default_portable")]
    pub portable: bool,
    /// Existing erasure timestamp, if already tombstoned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub erased_at: Option<DateTime<Utc>>,
}

/// Article 17 tombstone returned for erased records.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ErasureTombstone {
    /// The record that must be replaced by a tombstone by the caller.
    pub record_id: String,
    /// When the erasure decision was produced.
    pub erased_at: DateTime<Utc>,
}

/// Response shape returned by every supported right handler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSubjectResponse {
    /// The original request being answered.
    pub request: DataSubjectRequest,
    /// When the controller produced the response.
    pub responded_at: DateTime<Utc>,
    /// Operation-specific structured payload.
    pub payload: Value,
    /// Audit-chain sequence number anchoring this response, when recorded.
    pub audit_seq: Option<u64>,
}

/// Error returned by GDPR handlers.
#[derive(Debug, Error)]
pub enum GdprError {
    /// The requested right is represented in the contract but not handled here.
    #[error("gdpr right is not supported by this handler")]
    UnsupportedRight,
    /// The subject identifier was empty.
    #[error("data subject id is empty")]
    EmptySubject,
    /// Serialisation failed.
    #[error("serialisation error: {0}")]
    Serde(#[from] serde_json::Error),
}

/// Handle a data-subject request with no caller-supplied records.
pub fn handle_request(request: &DataSubjectRequest) -> Result<DataSubjectResponse, GdprError> {
    handle_request_with_records(request, &[])
}

/// Handle Article 15, 17, and 20 for caller-supplied subject records.
pub fn handle_request_with_records(
    request: &DataSubjectRequest,
    records: &[DataSubjectRecord],
) -> Result<DataSubjectResponse, GdprError> {
    if request.subject.0.trim().is_empty() {
        return Err(GdprError::EmptySubject);
    }
    let responded_at = Utc::now();
    let payload = match request.right {
        GdprRight::Access => access_payload(records),
        GdprRight::Erasure => erasure_payload(records, responded_at),
        GdprRight::Portability => portability_payload(records),
        _ => return Err(GdprError::UnsupportedRight),
    };
    Ok(DataSubjectResponse {
        request: request.clone(),
        responded_at,
        payload,
        audit_seq: None,
    })
}

fn access_payload(records: &[DataSubjectRecord]) -> Value {
    let visible: Vec<_> = records
        .iter()
        .filter(|record| record.erased_at.is_none())
        .cloned()
        .collect();
    json!({"article":"15","record_count":visible.len(),"records":visible})
}

fn erasure_payload(records: &[DataSubjectRecord], erased_at: DateTime<Utc>) -> Value {
    let tombstones: Vec<_> = records
        .iter()
        .filter(|record| record.erased_at.is_none())
        .map(|record| ErasureTombstone {
            record_id: record.record_id.clone(),
            erased_at,
        })
        .collect();
    json!({"article":"17","erased_count":tombstones.len(),"tombstones":tombstones})
}

fn portability_payload(records: &[DataSubjectRecord]) -> Value {
    let portable: Vec<_> = records
        .iter()
        .filter(|record| record.portable && record.erased_at.is_none())
        .cloned()
        .collect();
    json!({"article":"20","format":"application/json","record_count":portable.len(),"records":portable})
}

fn default_portable() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(right: GdprRight) -> DataSubjectRequest {
        DataSubjectRequest {
            subject: DataSubjectId("subj-1".into()),
            right,
            received_at: Utc::now(),
            note: None,
        }
    }

    fn records() -> Vec<DataSubjectRecord> {
        vec![
            DataSubjectRecord {
                record_id: "rec-1".into(),
                namespace: "ontology.object".into(),
                payload: json!({"value":"alpha"}),
                portable: true,
                erased_at: None,
            },
            DataSubjectRecord {
                record_id: "rec-2".into(),
                namespace: "audit".into(),
                payload: json!({"value":"retained"}),
                portable: false,
                erased_at: None,
            },
        ]
    }

    #[test]
    fn article_15_access_exports_visible_records() {
        let res = handle_request_with_records(&request(GdprRight::Access), &records()).unwrap();
        assert_eq!(res.payload["article"], "15");
        assert_eq!(res.payload["record_count"], 2);
    }

    #[test]
    fn article_17_erasure_returns_tombstones() {
        let res = handle_request_with_records(&request(GdprRight::Erasure), &records()).unwrap();
        assert_eq!(res.payload["article"], "17");
        assert_eq!(res.payload["erased_count"], 2);
    }

    #[test]
    fn article_20_portability_filters_nonportable_records() {
        let res =
            handle_request_with_records(&request(GdprRight::Portability), &records()).unwrap();
        assert_eq!(res.payload["article"], "20");
        assert_eq!(res.payload["record_count"], 1);
    }
}
