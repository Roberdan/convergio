//! # convergio-ontology-author
//!
//! LLM-assisted ontology authoring (ADR-0080). Given **documents**
//! and/or a generic **intent** (a prompt + industry + use-case), this
//! crate produces a domain ontology as a set of **standard artifacts**
//! usable by Convergio *and* external tools:
//!
//! | Artifact | Format | Consumer |
//! |----------|--------|----------|
//! | `owl/<name>.ttl`            | OWL 2 Turtle      | Protégé, RDF tools |
//! | `jsonschema/<Obj>.schema.json` | JSON-Schema     | validators, codegen |
//! | `shacl/<Obj>.shacl.jsonld`  | SHACL JSON-LD     | RDF validators |
//! | `ontology.json`             | Convergio draft   | registry import |
//! | `provenance.json`           | W3C PROV-JSON     | audit |
//!
//! ## Thesis, not a wrapper
//!
//! The machine must *prove* its output. The pipeline constrains the LLM
//! to a JSON-Schema, validates the result (RDF-safe names, datatype
//! allowlist, link/property closure), runs a bounded repair loop, and
//! records provenance for every source document. It **never** writes to
//! the ontology registry — it emits a draft for human review.
//!
//! ## Boundaries
//!
//! Leaf crate: nothing else depends on it. Per ADR-0032 the LLM step
//! shells out to the operator's vendor CLI (never a raw HTTP API).
//! Document conversion uses `markitdown` (never LibreOffice).

#![forbid(unsafe_code)]

mod author;
#[cfg(test)]
mod author_tests;
mod draft;
mod draft_names;
mod emit;
mod error;
mod ingest;
mod intent;
mod owl;
mod prompt;
mod propose;
mod provenance;
mod records;
mod validate;

pub use author::{author, AuthorOptions, AuthoringOutcome};
pub use draft::{DraftLink, DraftObject, DraftOntology, DraftProperty};
pub use draft_names::{is_valid_name, normalize_datatype, CANONICAL_DATATYPES};
pub use emit::{write_artifacts, ArtifactSet};
pub use error::{AuthorError, Result};
pub use ingest::{ingest_all, DocConverter, MarkitdownConverter, SourceDoc};
pub use intent::{AuthoringRequest, Intent};
pub use owl::to_owl_turtle;
pub use propose::{CliProposer, OntologyProposer};
pub use provenance::{build_bundle, bundle_json};
pub use records::{build_records, OntologyRecords};
pub use validate::{render_violations, validate, Violation};
