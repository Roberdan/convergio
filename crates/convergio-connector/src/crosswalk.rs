//! YAML crosswalk mapping (source fields → ontology properties).

use crate::canonical_json::to_canonical_bytes;
use crate::error::ConnectorError;
use crate::types::SchemaHash;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Top-level YAML mapping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Crosswalk {
    /// Connector id the crosswalk is meant for.
    pub connector_id: String,

    /// Stable mapping schema version.
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,

    /// Field-level mappings.
    #[serde(default)]
    pub fields: Vec<CrosswalkField>,
}

fn default_schema_version() -> u32 {
    1
}

/// One mapped field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrosswalkField {
    /// Source field name (as emitted by the connector).
    pub source: String,

    /// Ontology property reference (stringly-typed in core).
    pub property: String,

    /// Optional comparator hint for entity resolution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comparator: Option<String>,

    /// Whether this field participates in the stable source key.
    #[serde(default)]
    pub source_key: bool,
}

/// Minimal parse report for observability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrosswalkParseReport {
    /// Total fields declared.
    pub field_count: usize,
    /// Number of fields flagged as part of `source_key`.
    pub source_key_fields: usize,
}

impl Crosswalk {
    /// Parse a YAML crosswalk from bytes.
    pub fn from_yaml_bytes(bytes: &[u8]) -> Result<(Self, CrosswalkParseReport), ConnectorError> {
        let cw: Crosswalk = serde_yaml::from_slice(bytes).map_err(ConnectorError::yaml)?;
        cw.validate()?;
        let report = CrosswalkParseReport {
            field_count: cw.fields.len(),
            source_key_fields: cw.fields.iter().filter(|f| f.source_key).count(),
        };
        Ok((cw, report))
    }

    /// Compute a stable schema hash over the canonical JSON form.
    pub fn schema_hash(&self) -> Result<SchemaHash, ConnectorError> {
        let bytes = to_canonical_bytes(self)?;
        let digest = Sha256::digest(bytes);
        Ok(SchemaHash::new_hex(hex::encode(digest)))
    }

    fn validate(&self) -> Result<(), ConnectorError> {
        if self.connector_id.trim().is_empty() {
            return Err(ConnectorError::protocol(
                "crosswalk.connector_id must be non-empty",
            ));
        }
        if self.fields.is_empty() {
            return Err(ConnectorError::protocol(
                "crosswalk.fields must be non-empty",
            ));
        }
        for f in &self.fields {
            if f.source.trim().is_empty() {
                return Err(ConnectorError::protocol(
                    "crosswalk.fields[].source must be non-empty",
                ));
            }
            if f.property.trim().is_empty() {
                return Err(ConnectorError::protocol(
                    "crosswalk.fields[].property must be non-empty",
                ));
            }
        }
        if !self.fields.iter().any(|f| f.source_key) {
            return Err(ConnectorError::protocol(
                "crosswalk must mark at least one field with source_key: true",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const YAML: &str = r#"
connector_id: "csv"
fields:
  - source: "id"
    property: "Person.external_id"
    source_key: true
  - source: "name"
    property: "Person.name"
    comparator: "token"
"#;

    #[test]
    fn parses_and_hashes_stably() {
        let (cw, report) = Crosswalk::from_yaml_bytes(YAML.as_bytes()).expect("parse");
        assert_eq!(report.field_count, 2);
        assert_eq!(report.source_key_fields, 1);
        let h1 = cw.schema_hash().expect("hash1");
        let h2 = cw.schema_hash().expect("hash2");
        assert_eq!(h1, h2);
        assert_eq!(h1.as_hex().len(), 64);
    }

    #[test]
    fn refuses_missing_source_key() {
        let bad = r#"connector_id: x
fields:
  - source: a
    property: b
"#;
        let err = Crosswalk::from_yaml_bytes(bad.as_bytes()).unwrap_err();
        assert!(err.to_string().contains("source_key"));
    }
}
