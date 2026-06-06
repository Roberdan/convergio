//! The LLM input/output contract: the `DraftOntology` schema.
//!
//! This is the shape the proposer must emit. It deliberately mirrors
//! the three ontology record families (object / link / property) but
//! exposes only authoring-relevant fields — versioning, hashes and
//! audit columns are owned by the store, not the LLM. The JSON-Schema
//! derived from these types is embedded in the prompt so the model is
//! constrained to a known structure (ADR-0080).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A proposed object type (a typed thing the domain talks about).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DraftObject {
    /// RDF-safe machine name, e.g. `Student`. Grammar: `^[A-Za-z][A-Za-z0-9_]*$`.
    pub name: String,
    /// Short human title.
    pub title: String,
    /// Longer human description.
    #[serde(default)]
    pub description: String,
}

/// A proposed property (a typed attribute of an object).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DraftProperty {
    /// RDF-safe machine name, e.g. `email`.
    pub name: String,
    /// Machine name of the owning object.
    pub owner: String,
    /// Datatype: one of string, integer, number, boolean, datetime,
    /// date, time, iri, uuid (aliases are normalized).
    pub datatype: String,
    /// Whether instances must carry this property.
    #[serde(default)]
    pub required: bool,
    /// Short human title.
    #[serde(default)]
    pub title: String,
    /// Longer human description.
    #[serde(default)]
    pub description: String,
}

/// A proposed link (a typed relation between two object types).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DraftLink {
    /// RDF-safe machine name, e.g. `enrolled_in`.
    pub name: String,
    /// Machine name of the source object.
    pub from: String,
    /// Machine name of the target object.
    pub to: String,
    /// Short human title.
    #[serde(default)]
    pub title: String,
    /// Longer human description.
    #[serde(default)]
    pub description: String,
}

/// A complete proposed ontology draft.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DraftOntology {
    /// RDF-safe ontology name used as the base IRI segment, e.g. `sis`.
    #[serde(default)]
    pub name: String,
    /// Object types.
    #[serde(default)]
    pub objects: Vec<DraftObject>,
    /// Property types.
    #[serde(default)]
    pub properties: Vec<DraftProperty>,
    /// Link types.
    #[serde(default)]
    pub links: Vec<DraftLink>,
}

impl DraftOntology {
    /// The JSON-Schema (draft 2020-12) for this type, as pretty bytes.
    /// Embedded in the prompt to constrain the proposer's output.
    pub fn json_schema_string() -> String {
        let schema = schemars::schema_for!(DraftOntology);
        serde_json::to_string_pretty(&schema)
            .unwrap_or_else(|_| "{\"type\":\"object\"}".to_string())
    }

    /// Parse a proposer's raw text into a draft, tolerating a leading
    /// ```` ```json ```` fence the way chat CLIs often wrap output.
    pub fn parse(raw: &str) -> std::result::Result<Self, serde_json::Error> {
        serde_json::from_str(extract_json(raw))
    }
}

/// Strip a Markdown code fence around a JSON body, if present, and
/// trim to the outermost `{ ... }` so trailing prose is ignored.
fn extract_json(raw: &str) -> &str {
    let trimmed = raw.trim();
    let body = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .map(|s| s.trim_start())
        .unwrap_or(trimmed);
    let body = body.strip_suffix("```").unwrap_or(body).trim();
    match (body.find('{'), body.rfind('}')) {
        (Some(start), Some(end)) if end >= start => &body[start..=end],
        _ => body,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tolerates_code_fence_and_prose() {
        let raw = "Here is the ontology:\n```json\n{\"name\":\"sis\",\"objects\":[]}\n```\nDone.";
        let d = DraftOntology::parse(raw).unwrap();
        assert_eq!(d.name, "sis");
    }

    #[test]
    fn schema_string_is_non_trivial() {
        let s = DraftOntology::json_schema_string();
        assert!(s.contains("objects"));
        assert!(s.contains("links"));
        assert!(s.contains("properties"));
    }
}
