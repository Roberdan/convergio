//! Deterministic prompt construction for the proposer.
//!
//! The prompt is assembled from the operator's intent, the grounding
//! text extracted from source documents, the `DraftOntology`
//! JSON-Schema (so the model is constrained to a known structure) and,
//! on repair turns, the list of violations from the previous attempt.
//! Document text is truncated to keep the prompt bounded.

use crate::draft::DraftOntology;
use crate::ingest::SourceDoc;
use crate::intent::Intent;
use crate::validate::{render_violations, Violation};

/// Max characters of grounding text taken from each document.
const PER_DOC_BUDGET: usize = 8_000;

/// Compose the full proposer prompt.
pub fn build_prompt(
    intent: Option<&Intent>,
    docs: &[SourceDoc],
    previous: Option<(&str, &[Violation])>,
) -> String {
    let mut p = String::new();
    p.push_str(
        "You are an ontology engineer. Design a domain ontology and return it \
         STRICTLY as a single JSON object that conforms to the JSON Schema below. \
         Output JSON ONLY — no prose, no Markdown fence.\n\n",
    );

    if let Some(i) = intent {
        p.push_str("## Intent\n");
        if !i.prompt.trim().is_empty() {
            p.push_str(&format!("Goal: {}\n", i.prompt.trim()));
        }
        if !i.industry.trim().is_empty() {
            p.push_str(&format!("Industry: {}\n", i.industry.trim()));
        }
        if !i.use_case.trim().is_empty() {
            p.push_str(&format!("Use case: {}\n", i.use_case.trim()));
        }
        p.push('\n');
    }

    if !docs.is_empty() {
        p.push_str("## Source documents (ground the ontology in these)\n");
        for d in docs {
            p.push_str(&format!("### {}\n", d.path.display()));
            p.push_str(truncate(&d.markdown, PER_DOC_BUDGET));
            p.push_str("\n\n");
        }
    }

    p.push_str("## Rules\n");
    p.push_str(
        "- Object/link/property `name` and the ontology `name` must match \
         ^[A-Za-z][A-Za-z0-9_]*$ (no spaces, no hyphens, not starting with a digit).\n\
         - Use PascalCase for object names, snake_case for property and link names.\n\
         - Property `datatype` must be one of: string, integer, number, boolean, \
         datetime, date, time, iri, uuid.\n\
         - Every property `owner` and every link `from`/`to` must reference a \
         defined object.\n\
         - Give every object and link a human `title`.\n\n",
    );

    if let Some((prev_json, violations)) = previous {
        p.push_str("## Your previous attempt failed validation\n");
        p.push_str("Previous JSON:\n");
        p.push_str(truncate(prev_json, PER_DOC_BUDGET));
        p.push_str("\n\nFix these violations and return corrected JSON only:\n");
        p.push_str(&render_violations(violations));
        p.push('\n');
    }

    p.push_str("## JSON Schema\n");
    p.push_str(&DraftOntology::json_schema_string());
    p.push('\n');
    p
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    // Snap to a char boundary at or below `max`.
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_includes_schema_and_intent() {
        let intent = Intent {
            prompt: "model a university".into(),
            industry: "higher-education".into(),
            use_case: "sis".into(),
        };
        let p = build_prompt(Some(&intent), &[], None);
        assert!(p.contains("model a university"));
        assert!(p.contains("JSON Schema"));
        assert!(p.contains("\"objects\""));
    }

    #[test]
    fn repair_prompt_lists_violations() {
        let v = vec![Violation {
            locus: "link[x]".into(),
            message: "to 'Ghost' is not a defined object".into(),
        }];
        let p = build_prompt(None, &[], Some(("{}", &v)));
        assert!(p.contains("failed validation"));
        assert!(p.contains("Ghost"));
    }
}
