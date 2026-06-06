//! Minimal, deterministic OWL 2 emitter in Turtle syntax.
//!
//! This is the artifact external ontology tools (e.g. Protégé) open. We
//! emit a deliberately small profile — `owl:Ontology`, `owl:Class`,
//! `owl:ObjectProperty`, `owl:DatatypeProperty` with `rdfs:domain` /
//! `rdfs:range` / `rdfs:label` / `rdfs:comment` — which is enough to
//! round-trip the registry's object/link/property model without
//! reaching for OWL restrictions. Output is one coherent graph (not
//! per-object files) and is byte-deterministic for a given input.

use crate::records::OntologyRecords;

/// Build the base IRI for an ontology of the given name.
fn base_iri(name: &str) -> String {
    format!("https://convergio.dev/ontology/{name}#")
}

/// Map a canonical ontology datatype to its `xsd:` range local name.
fn xsd_range(datatype: &str) -> &'static str {
    match datatype {
        "integer" => "xsd:integer",
        "number" => "xsd:decimal",
        "boolean" => "xsd:boolean",
        "datetime" => "xsd:dateTime",
        "date" => "xsd:date",
        "time" => "xsd:time",
        "iri" => "xsd:anyURI",
        // string, uuid and any normalized-but-unmapped value
        _ => "xsd:string",
    }
}

/// Escape a string for a Turtle double-quoted literal.
fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out
}

fn label_comment(out: &mut String, title: &str, description: &str) {
    if !title.is_empty() {
        out.push_str(&format!("    rdfs:label \"{}\" ;\n", esc(title)));
    }
    if !description.is_empty() {
        out.push_str(&format!("    rdfs:comment \"{}\" ;\n", esc(description)));
    }
}

/// Render the whole ontology as an OWL 2 Turtle document.
pub fn to_owl_turtle(name: &str, records: &OntologyRecords) -> String {
    let base = base_iri(name);
    let mut out = String::new();
    out.push_str("@prefix : <");
    out.push_str(&base);
    out.push_str("> .\n");
    out.push_str("@prefix owl: <http://www.w3.org/2002/07/owl#> .\n");
    out.push_str("@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n");
    out.push_str("@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n\n");

    out.push_str(&format!("<{base}> a owl:Ontology .\n\n"));

    for o in &records.objects {
        out.push_str(&format!(":{} a owl:Class ;\n", o.name));
        label_comment(&mut out, &o.title, &o.description);
        terminate(&mut out);
    }

    for p in &records.properties {
        out.push_str(&format!(
            ":{}_{} a owl:DatatypeProperty ;\n",
            p.owner_name, p.name
        ));
        label_comment(&mut out, &p.title, &p.description);
        out.push_str(&format!("    rdfs:domain :{} ;\n", p.owner_name));
        out.push_str(&format!("    rdfs:range {} ;\n", xsd_range(&p.datatype)));
        terminate(&mut out);
    }

    for l in &records.links {
        out.push_str(&format!(":{} a owl:ObjectProperty ;\n", l.name));
        label_comment(&mut out, &l.title, &l.description);
        out.push_str(&format!("    rdfs:domain :{} ;\n", l.from_object));
        out.push_str(&format!("    rdfs:range :{} ;\n", l.to_object));
        terminate(&mut out);
    }

    out
}

/// Replace the trailing ` ;\n` of the last predicate with ` .\n\n`.
fn terminate(out: &mut String) {
    if out.ends_with(" ;\n") {
        out.truncate(out.len() - 3);
        out.push_str(" .\n\n");
    } else {
        out.push_str(".\n\n");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draft::{DraftLink, DraftObject, DraftOntology, DraftProperty};
    use crate::records::build_records;
    use chrono::{TimeZone, Utc};

    fn records() -> OntologyRecords {
        let d = DraftOntology {
            name: "sis".into(),
            objects: vec![DraftObject {
                name: "Student".into(),
                title: "Student".into(),
                description: "A learner".into(),
            }],
            properties: vec![DraftProperty {
                name: "email".into(),
                owner: "Student".into(),
                datatype: "string".into(),
                required: true,
                title: "Email".into(),
                description: String::new(),
            }],
            links: vec![DraftLink {
                name: "advised_by".into(),
                from: "Student".into(),
                to: "Student".into(),
                title: "Advised by".into(),
                description: String::new(),
            }],
        };
        build_records(&d, Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()).unwrap()
    }

    #[test]
    fn emits_classes_and_properties() {
        let ttl = to_owl_turtle("sis", &records());
        assert!(ttl.contains(":Student a owl:Class"));
        assert!(ttl.contains(":Student_email a owl:DatatypeProperty"));
        assert!(ttl.contains("rdfs:range xsd:string"));
        assert!(ttl.contains(":advised_by a owl:ObjectProperty"));
        assert!(ttl.contains("rdfs:domain :Student"));
        assert!(ttl.trim_end().ends_with("."));
    }
}
