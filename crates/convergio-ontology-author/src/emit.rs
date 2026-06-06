//! Write the standard artifacts to an output directory.
//!
//! For each object we emit the runtime's deterministic JSON-Schema and
//! SHACL (the latter is **SHACL JSON-LD**, hence the `.shacl.jsonld`
//! extension — not Turtle). The whole ontology is emitted once as OWL 2
//! Turtle (`owl/<name>.ttl`), the convergio-native draft as
//! `ontology.json`, and provenance as `provenance.json`. Every path is
//! returned so the CLI can report exactly what was produced.

use std::fs;
use std::path::{Path, PathBuf};

use convergio_ontology::{build_object_schema_bytes, build_object_shacl_bytes};

use crate::draft::DraftOntology;
use crate::error::{AuthorError, Result};
use crate::owl::to_owl_turtle;
use crate::records::OntologyRecords;

/// Paths of everything written by [`write_artifacts`].
#[derive(Debug, Clone, Default)]
pub struct ArtifactSet {
    /// Combined OWL 2 Turtle document.
    pub owl: PathBuf,
    /// Per-object JSON-Schema files.
    pub json_schema: Vec<PathBuf>,
    /// Per-object SHACL JSON-LD files.
    pub shacl: Vec<PathBuf>,
    /// Convergio-native draft JSON (for later registry import).
    pub ontology_json: PathBuf,
    /// PROV-JSON provenance bundle.
    pub provenance: PathBuf,
}

fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| AuthorError::Write {
            path: parent.to_path_buf(),
            reason: e.to_string(),
        })?;
    }
    fs::write(path, bytes).map_err(|e| AuthorError::Write {
        path: path.to_path_buf(),
        reason: e.to_string(),
    })
}

/// Render and write every artifact under `out`. Returns their paths.
pub fn write_artifacts(
    out: &Path,
    draft: &DraftOntology,
    records: &OntologyRecords,
    provenance_json: &[u8],
) -> Result<ArtifactSet> {
    // OWL (whole-graph).
    let owl = out.join("owl").join(format!("{}.ttl", draft.name));
    write(&owl, to_owl_turtle(&draft.name, records).as_bytes())?;

    // Per-object JSON-Schema + SHACL JSON-LD.
    let mut json_schema = Vec::new();
    let mut shacl = Vec::new();
    for object in &records.objects {
        let props = records.properties_of(&object.name);
        let props_owned: Vec<_> = props.into_iter().cloned().collect();

        let schema = build_object_schema_bytes(object, &props_owned)
            .map_err(|e| AuthorError::Ontology(e.to_string()))?;
        let schema_path = out
            .join("jsonschema")
            .join(format!("{}.schema.json", object.name));
        write(&schema_path, &schema)?;
        json_schema.push(schema_path);

        let shacl_bytes = build_object_shacl_bytes(object, &props_owned)
            .map_err(|e| AuthorError::Ontology(e.to_string()))?;
        let shacl_path = out
            .join("shacl")
            .join(format!("{}.shacl.jsonld", object.name));
        write(&shacl_path, &shacl_bytes)?;
        shacl.push(shacl_path);
    }

    // Convergio-native draft + provenance.
    let ontology_json = out.join("ontology.json");
    let draft_bytes = serde_json::to_vec_pretty(draft).map_err(|e| AuthorError::Write {
        path: ontology_json.clone(),
        reason: e.to_string(),
    })?;
    write(&ontology_json, &draft_bytes)?;

    let provenance = out.join("provenance.json");
    write(&provenance, provenance_json)?;

    Ok(ArtifactSet {
        owl,
        json_schema,
        shacl,
        ontology_json,
        provenance,
    })
}
