//! Deterministic Mermaid and Graphviz (DOT) renderers for the
//! ontology diff / lineage surfaces (ADR-0060, W1 T9).
//!
//! Byte-identity rules:
//!
//! - All collections rendered are already sorted at the diff /
//!   lineage layer.
//! - No timestamps, no environment, no random ids.
//! - Mermaid abbreviates content hashes to 7 characters; DOT keeps
//!   the full hash (consumers of DOT often re-hash anyway).
//! - Line endings are `\n` and the document ends with exactly one
//!   trailing newline.

use crate::diff::ObjectDiff;
use crate::lineage::Lineage;
use std::fmt::Write as _;

const HASH_ABBREV: usize = 7;

fn abbrev(h: &str) -> &str {
    if h.len() > HASH_ABBREV {
        &h[..HASH_ABBREV]
    } else {
        h
    }
}

/// Render an [`ObjectDiff`] as Mermaid `flowchart LR`. Stable for
/// the same diff input.
pub fn render_diff_mermaid(d: &ObjectDiff) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "%% ontology diff");
    let _ = writeln!(
        out,
        "%% object={} from={} to={}",
        d.object_name, d.from_version, d.to_version
    );
    let _ = writeln!(out, "flowchart LR");
    let from_hash = d
        .from_object_hash
        .as_deref()
        .map(abbrev)
        .unwrap_or("absent");
    let to_hash = d.to_object_hash.as_deref().map(abbrev).unwrap_or("absent");
    let _ = writeln!(
        out,
        "  obj_from[\"{name} v{v} ({h})\"]",
        name = d.object_name,
        v = d.from_version,
        h = from_hash
    );
    let _ = writeln!(
        out,
        "  obj_to[\"{name} v{v} ({h})\"]",
        name = d.object_name,
        v = d.to_version,
        h = to_hash
    );
    let edge = if d.object_changed { "==>" } else { "-->" };
    let _ = writeln!(out, "  obj_from {} obj_to", edge);
    for p in &d.added {
        let _ = writeln!(
            out,
            "  obj_to -->|added| add_{name}[\"{name} ({h})\"]",
            name = p.name,
            h = abbrev(&p.content_hash)
        );
    }
    for p in &d.removed {
        let _ = writeln!(
            out,
            "  obj_from -->|removed| rem_{name}[\"{name} ({h})\"]",
            name = p.name,
            h = abbrev(&p.content_hash)
        );
    }
    for p in &d.modified {
        let _ = writeln!(
            out,
            "  obj_to -->|modified {from}→{to}| mod_{name}[\"{name}\"]",
            name = p.name,
            from = abbrev(&p.from_hash),
            to = abbrev(&p.to_hash)
        );
    }
    out
}

/// Render an [`ObjectDiff`] as Graphviz DOT.
pub fn render_diff_dot(d: &ObjectDiff) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "// ontology diff");
    let _ = writeln!(
        out,
        "// object={} from={} to={}",
        d.object_name, d.from_version, d.to_version
    );
    let _ = writeln!(out, "digraph ontology_diff {{");
    let _ = writeln!(out, "  rankdir=LR;");
    let from_hash = d.from_object_hash.as_deref().unwrap_or("absent");
    let to_hash = d.to_object_hash.as_deref().unwrap_or("absent");
    let _ = writeln!(
        out,
        "  \"obj_from\" [label=\"{name} v{v}\\n{h}\"];",
        name = d.object_name,
        v = d.from_version,
        h = from_hash
    );
    let _ = writeln!(
        out,
        "  \"obj_to\" [label=\"{name} v{v}\\n{h}\"];",
        name = d.object_name,
        v = d.to_version,
        h = to_hash
    );
    let style = if d.object_changed { "bold" } else { "dashed" };
    let _ = writeln!(out, "  \"obj_from\" -> \"obj_to\" [style={}];", style);
    for p in &d.added {
        let _ = writeln!(
            out,
            "  \"add_{name}\" [label=\"{name}\\n{h}\" color=green];",
            name = p.name,
            h = p.content_hash
        );
        let _ = writeln!(
            out,
            "  \"obj_to\" -> \"add_{name}\" [label=\"added\"];",
            name = p.name
        );
    }
    for p in &d.removed {
        let _ = writeln!(
            out,
            "  \"rem_{name}\" [label=\"{name}\\n{h}\" color=red];",
            name = p.name,
            h = p.content_hash
        );
        let _ = writeln!(
            out,
            "  \"obj_from\" -> \"rem_{name}\" [label=\"removed\"];",
            name = p.name
        );
    }
    for p in &d.modified {
        let _ = writeln!(
            out,
            "  \"mod_{name}\" [label=\"{name}\\n{from}->{to}\" color=orange];",
            name = p.name,
            from = p.from_hash,
            to = p.to_hash
        );
        let _ = writeln!(
            out,
            "  \"obj_to\" -> \"mod_{name}\" [label=\"modified\"];",
            name = p.name
        );
    }
    let _ = writeln!(out, "}}");
    out
}

/// Render a [`Lineage`] as a linear Mermaid `flowchart LR`.
pub fn render_lineage_mermaid(l: &Lineage) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "%% ontology lineage");
    let _ = writeln!(out, "%% object={}", l.object_name);
    let _ = writeln!(out, "flowchart LR");
    for n in &l.nodes {
        let marker = if n.breaking { "!" } else { "" };
        let _ = writeln!(
            out,
            "  v{v}[\"{name} v{v}{m} ({h})\"]",
            v = n.schema_version,
            name = l.object_name,
            m = marker,
            h = abbrev(&n.content_hash)
        );
    }
    for win in l.nodes.windows(2) {
        let edge = if win[1].breaking { "==>" } else { "-->" };
        let _ = writeln!(
            out,
            "  v{a} {edge} v{b}",
            a = win[0].schema_version,
            b = win[1].schema_version,
            edge = edge
        );
    }
    out
}

/// Render a [`Lineage`] as Graphviz DOT.
pub fn render_lineage_dot(l: &Lineage) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "// ontology lineage");
    let _ = writeln!(out, "// object={}", l.object_name);
    let _ = writeln!(out, "digraph ontology_lineage {{");
    let _ = writeln!(out, "  rankdir=LR;");
    for n in &l.nodes {
        let marker = if n.breaking { " (breaking)" } else { "" };
        let _ = writeln!(
            out,
            "  \"v{v}\" [label=\"{name} v{v}{m}\\n{h}\"];",
            v = n.schema_version,
            name = l.object_name,
            m = marker,
            h = n.content_hash
        );
    }
    for win in l.nodes.windows(2) {
        let style = if win[1].breaking { "bold" } else { "solid" };
        let _ = writeln!(
            out,
            "  \"v{a}\" -> \"v{b}\" [style={style}];",
            a = win[0].schema_version,
            b = win[1].schema_version,
            style = style
        );
    }
    let _ = writeln!(out, "}}");
    out
}
