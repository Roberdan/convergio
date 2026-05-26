use crate::TypeId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// External ontology namespaces we expect verticals to map against.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExternalSystem {
    /// Common Education Data Standards (CEDS).
    Ceds,
    /// European Learning Model (ELMO).
    Elmo,
    /// European Skills, Competences, Qualifications and Occupations (ESCO).
    Esco,
}

impl ExternalSystem {
    /// Stable lowercase label.
    pub fn as_str(self) -> &'static str {
        match self {
            ExternalSystem::Ceds => "ceds",
            ExternalSystem::Elmo => "elmo",
            ExternalSystem::Esco => "esco",
        }
    }
}

/// One mapping row between an external IRI and an internal schema type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct IriMappingRow {
    /// Which external system owns the IRI.
    pub system: ExternalSystem,
    /// External IRI (http/https).
    pub external_iri: String,
    /// Which internal kind the mapping points to.
    pub internal_kind: InternalKind,
    /// Internal type id.
    pub internal_id: TypeId,
    /// Freeform note.
    pub note: Option<String>,
}

/// Which kind of schema element the mapping points to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InternalKind {
    /// Maps to an [`ObjectType`](crate::ObjectType) id.
    ObjectType,
    /// Maps to a [`LinkType`](crate::LinkType) id.
    LinkType,
    /// Maps to a [`PropertyType`](crate::PropertyType) id.
    PropertyType,
}

/// Validated mapping table.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct IriMappingTable {
    /// Mapping rows.
    pub rows: Vec<IriMappingRow>,
}

impl IriMappingTable {
    /// Create an empty mapping table.
    pub fn new() -> Self {
        Self { rows: Vec::new() }
    }

    /// Insert a new mapping row (validates IRI shape and uniqueness).
    pub fn insert(&mut self, row: IriMappingRow) -> Result<(), IriMappingError> {
        if !row.external_iri.starts_with("http://") && !row.external_iri.starts_with("https://") {
            return Err(IriMappingError::InvalidIri {
                iri: row.external_iri,
            });
        }
        if self.rows.iter().any(|r| r.external_iri == row.external_iri) {
            return Err(IriMappingError::DuplicateExternalIri {
                iri: row.external_iri,
            });
        }
        self.rows.push(row);
        Ok(())
    }

    /// Lookup a row by external IRI.
    pub fn by_external_iri(&self, iri: &str) -> Option<&IriMappingRow> {
        self.rows.iter().find(|r| r.external_iri == iri)
    }
}

/// Mapping table validation failures.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum IriMappingError {
    /// The `external_iri` was not an http/https URL.
    #[error("invalid IRI (expected http/https): {iri}")]
    InvalidIri {
        /// The invalid IRI.
        iri: String,
    },

    /// The `external_iri` is already present in the table.
    #[error("duplicate external IRI: {iri}")]
    DuplicateExternalIri {
        /// The duplicate IRI.
        iri: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_http_iri() {
        let mut t = IriMappingTable::new();
        let err = t
            .insert(IriMappingRow {
                system: ExternalSystem::Ceds,
                external_iri: "urn:bad".to_string(),
                internal_kind: InternalKind::PropertyType,
                internal_id: "prop.name".parse().unwrap(),
                note: None,
            })
            .unwrap_err();
        assert_eq!(
            err,
            IriMappingError::InvalidIri {
                iri: "urn:bad".to_string()
            }
        );
    }
}
