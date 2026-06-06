//! The authoring orchestrator: request in, draft artifacts out.
//!
//! Flow (ADR-0080): ingest documents → compose prompt → propose →
//! parse → validate → bounded repair loop → build records → provenance
//! → write artifacts. The pipeline NEVER writes to the ontology
//! registry; it only emits a reviewable draft. The clock and attempt
//! budget are injected so runs are deterministic and testable.

use std::path::PathBuf;

use chrono::{DateTime, Utc};

use crate::draft::DraftOntology;
use crate::emit::{write_artifacts, ArtifactSet};
use crate::error::{AuthorError, Result};
use crate::ingest::{ingest_all, DocConverter};
use crate::intent::AuthoringRequest;
use crate::prompt::build_prompt;
use crate::propose::OntologyProposer;
use crate::provenance::{build_bundle, bundle_json};
use crate::records::build_records;
use crate::validate::{validate, Violation};

/// Tunables for an authoring run.
pub struct AuthorOptions {
    /// Output directory for artifacts.
    pub out: PathBuf,
    /// Max proposer attempts (1 = no repair). Clamped to >= 1.
    pub max_attempts: u32,
    /// Wall-clock stamp applied to records and provenance.
    pub now: DateTime<Utc>,
}

impl AuthorOptions {
    /// Defaults: single attempt disabled — 3 attempts, now = `Utc::now`.
    pub fn new(out: PathBuf) -> Self {
        Self {
            out,
            max_attempts: 3,
            now: Utc::now(),
        }
    }
}

/// The result of a successful authoring run.
#[derive(Debug)]
pub struct AuthoringOutcome {
    /// The validated draft ontology.
    pub draft: DraftOntology,
    /// Paths of every artifact written.
    pub artifacts: ArtifactSet,
    /// Proposer attempts actually made.
    pub attempts: u32,
    /// Model identifier recorded in provenance.
    pub model_id: String,
}

/// Run the full authoring pipeline.
pub fn author(
    request: &AuthoringRequest,
    converter: &dyn DocConverter,
    proposer: &dyn OntologyProposer,
    opts: &AuthorOptions,
) -> Result<AuthoringOutcome> {
    request.ensure_non_empty()?;
    let docs = ingest_all(converter, &request.documents)?;
    let intent = request.intent.as_ref();
    let budget = opts.max_attempts.max(1);

    let mut attempts = 0;
    let mut last_json = String::new();
    let mut last_violations = Vec::new();
    let mut draft: Option<DraftOntology> = None;

    while attempts < budget {
        attempts += 1;
        let previous = if attempts == 1 {
            None
        } else {
            Some((last_json.as_str(), last_violations.as_slice()))
        };
        let prompt = build_prompt(intent, &docs, previous);
        let raw = proposer.propose(&prompt)?;
        let parsed = match DraftOntology::parse(&raw) {
            Ok(d) => d,
            Err(e) => {
                // Treat unparseable output as a violation and retry.
                last_json = raw;
                last_violations = vec![Violation {
                    locus: "output".to_string(),
                    message: format!("output was not valid JSON: {e}"),
                }];
                if attempts >= budget {
                    return Err(AuthorError::Parse(e.to_string()));
                }
                continue;
            }
        };
        let violations = validate(&parsed);
        if violations.is_empty() {
            draft = Some(parsed);
            break;
        }
        last_json = serde_json::to_string(&parsed).unwrap_or_default();
        last_violations = violations;
    }

    let draft = draft.ok_or(AuthorError::Unrepaired {
        attempts,
        count: last_violations.len(),
    })?;

    let records = build_records(&draft, opts.now)?;
    let bundle = build_bundle(&draft.name, &proposer.model_id(), &docs, opts.now)?;
    let prov_json = bundle_json(&bundle)?;
    let artifacts = write_artifacts(&opts.out, &draft, &records, &prov_json)?;

    Ok(AuthoringOutcome {
        draft,
        artifacts,
        attempts,
        model_id: proposer.model_id(),
    })
}
