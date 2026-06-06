//! Structural validation of a [`DraftOntology`] before export.
//!
//! These checks run after every proposer attempt. They catch the
//! failure modes an LLM produces: unsafe names, dangling link/property
//! owners, duplicate names, unknown datatypes and empty titles. Any
//! violation feeds the repair loop (re-prompt) or, after the attempt
//! budget, a hard failure — the tool never emits an invalid ontology.

use std::collections::BTreeSet;

use crate::draft::DraftOntology;
use crate::draft_names::{is_valid_name, normalize_datatype};

/// A single, human-readable validation problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// Where the problem is (e.g. `object[Student]`, `link[enrolled_in].to`).
    pub locus: String,
    /// What is wrong.
    pub message: String,
}

impl Violation {
    fn new(locus: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            locus: locus.into(),
            message: message.into(),
        }
    }
}

/// Validate a draft, returning every violation found (empty == valid).
/// Deterministic: results are produced in document order.
pub fn validate(draft: &DraftOntology) -> Vec<Violation> {
    let mut v = Vec::new();

    if draft.name.is_empty() || !is_valid_name(&draft.name) {
        v.push(Violation::new(
            "ontology.name",
            format!(
                "ontology name '{}' must match ^[A-Za-z][A-Za-z0-9_]*$",
                draft.name
            ),
        ));
    }

    let mut object_names: BTreeSet<&str> = BTreeSet::new();
    let mut seen_objects: BTreeSet<&str> = BTreeSet::new();
    for o in &draft.objects {
        let locus = format!("object[{}]", o.name);
        if !is_valid_name(&o.name) {
            v.push(Violation::new(
                &locus,
                "name is not a valid RDF-safe identifier",
            ));
        }
        if o.title.trim().is_empty() {
            v.push(Violation::new(&locus, "title must not be empty"));
        }
        if !seen_objects.insert(o.name.as_str()) {
            v.push(Violation::new(&locus, "duplicate object name"));
        }
        object_names.insert(o.name.as_str());
    }

    let mut seen_props: BTreeSet<(&str, &str)> = BTreeSet::new();
    for p in &draft.properties {
        let locus = format!("property[{}.{}]", p.owner, p.name);
        if !is_valid_name(&p.name) {
            v.push(Violation::new(
                &locus,
                "name is not a valid RDF-safe identifier",
            ));
        }
        if !object_names.contains(p.owner.as_str()) {
            v.push(Violation::new(
                &locus,
                format!("owner '{}' is not a defined object", p.owner),
            ));
        }
        if normalize_datatype(&p.datatype).is_none() {
            v.push(Violation::new(
                &locus,
                format!("datatype '{}' is not a recognised type", p.datatype),
            ));
        }
        if !seen_props.insert((p.owner.as_str(), p.name.as_str())) {
            v.push(Violation::new(&locus, "duplicate property on this owner"));
        }
    }

    let mut seen_links: BTreeSet<&str> = BTreeSet::new();
    for l in &draft.links {
        let locus = format!("link[{}]", l.name);
        if !is_valid_name(&l.name) {
            v.push(Violation::new(
                &locus,
                "name is not a valid RDF-safe identifier",
            ));
        }
        if l.title.trim().is_empty() {
            v.push(Violation::new(&locus, "title must not be empty"));
        }
        if !object_names.contains(l.from.as_str()) {
            v.push(Violation::new(
                &locus,
                format!("from '{}' is not a defined object", l.from),
            ));
        }
        if !object_names.contains(l.to.as_str()) {
            v.push(Violation::new(
                &locus,
                format!("to '{}' is not a defined object", l.to),
            ));
        }
        if !seen_links.insert(l.name.as_str()) {
            v.push(Violation::new(&locus, "duplicate link name"));
        }
    }

    if draft.objects.is_empty() {
        v.push(Violation::new(
            "ontology",
            "must define at least one object",
        ));
    }

    v
}

/// Render violations as a numbered block for re-prompting.
pub fn render_violations(violations: &[Violation]) -> String {
    let mut out = String::new();
    for (i, vi) in violations.iter().enumerate() {
        out.push_str(&format!("{}. {}: {}\n", i + 1, vi.locus, vi.message));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draft::{DraftLink, DraftObject, DraftProperty};

    fn good() -> DraftOntology {
        DraftOntology {
            name: "sis".into(),
            objects: vec![
                DraftObject {
                    name: "Student".into(),
                    title: "Student".into(),
                    description: String::new(),
                },
                DraftObject {
                    name: "Course".into(),
                    title: "Course".into(),
                    description: String::new(),
                },
            ],
            properties: vec![DraftProperty {
                name: "email".into(),
                owner: "Student".into(),
                datatype: "string".into(),
                required: true,
                title: "Email".into(),
                description: String::new(),
            }],
            links: vec![DraftLink {
                name: "enrolled_in".into(),
                from: "Student".into(),
                to: "Course".into(),
                title: "Enrolled in".into(),
                description: String::new(),
            }],
        }
    }

    #[test]
    fn clean_draft_has_no_violations() {
        assert!(validate(&good()).is_empty());
    }

    #[test]
    fn flags_dangling_link_and_bad_datatype() {
        let mut d = good();
        d.links[0].to = "Ghost".into();
        d.properties[0].datatype = "blob".into();
        let vs = validate(&d);
        assert!(vs
            .iter()
            .any(|v| v.message.contains("not a defined object")));
        assert!(vs
            .iter()
            .any(|v| v.message.contains("not a recognised type")));
    }
}
