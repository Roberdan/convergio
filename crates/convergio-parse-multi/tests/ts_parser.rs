//! Integration tests for the TypeScript parser (F2-2).
//!
//! Each test loads `tests/fixtures/ts/sample.ts` and asserts the
//! `(Vec<Node>, Vec<Edge>)` output against known declarations in that file.
//! Using a file fixture (rather than inline bytes) validates the full
//! read-from-disk path used by the fleet graph builder.

use convergio_graph::model::{EdgeKind, NodeKind};
use convergio_parse_multi::parse_ts;

fn fixture_bytes() -> Vec<u8> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/ts/sample.ts");
    std::fs::read(path).expect("fixture file must exist at tests/fixtures/ts/sample.ts")
}

const REPO: &str = "test-repo";
const FILE: &str = "tests/fixtures/ts/sample.ts";

// ── Module node ───────────────────────────────────────────────────────────────

#[test]
fn fixture_has_module_node() {
    let (nodes, _) = parse_ts(REPO, FILE, &fixture_bytes()).unwrap();
    let module = nodes.iter().find(|n| n.kind == NodeKind::Module);
    assert!(module.is_some(), "module node must be present");
    assert_eq!(module.unwrap().crate_name, REPO);
    assert_eq!(module.unwrap().file_path.as_deref(), Some(FILE));
}

// ── Item nodes ────────────────────────────────────────────────────────────────

#[test]
fn fixture_extracts_function_greet() {
    let (nodes, _) = parse_ts(REPO, FILE, &fixture_bytes()).unwrap();
    let n = nodes
        .iter()
        .find(|n| n.kind == NodeKind::Item && n.name == "greet");
    assert!(n.is_some(), "'greet' must be extracted");
    assert_eq!(n.unwrap().item_kind, Some("function"));
}

#[test]
fn fixture_extracts_class_animal() {
    let (nodes, _) = parse_ts(REPO, FILE, &fixture_bytes()).unwrap();
    let n = nodes
        .iter()
        .find(|n| n.kind == NodeKind::Item && n.name == "Animal");
    assert!(n.is_some(), "'Animal' must be extracted");
    assert_eq!(n.unwrap().item_kind, Some("class"));
}

#[test]
fn fixture_extracts_interface_describable() {
    let (nodes, _) = parse_ts(REPO, FILE, &fixture_bytes()).unwrap();
    let n = nodes
        .iter()
        .find(|n| n.kind == NodeKind::Item && n.name == "Describable");
    assert!(n.is_some(), "'Describable' must be extracted");
    assert_eq!(n.unwrap().item_kind, Some("interface"));
}

#[test]
fn fixture_extracts_type_point() {
    let (nodes, _) = parse_ts(REPO, FILE, &fixture_bytes()).unwrap();
    let n = nodes
        .iter()
        .find(|n| n.kind == NodeKind::Item && n.name == "Point");
    assert!(n.is_some(), "'Point' type alias must be extracted");
    assert_eq!(n.unwrap().item_kind, Some("type"));
}

#[test]
fn fixture_extracts_enum_direction() {
    let (nodes, _) = parse_ts(REPO, FILE, &fixture_bytes()).unwrap();
    let n = nodes
        .iter()
        .find(|n| n.kind == NodeKind::Item && n.name == "Direction");
    assert!(n.is_some(), "'Direction' enum must be extracted");
    assert_eq!(n.unwrap().item_kind, Some("enum"));
}

// ── Exported declarations ─────────────────────────────────────────────────────

#[test]
fn fixture_extracts_exported_function() {
    let (nodes, _) = parse_ts(REPO, FILE, &fixture_bytes()).unwrap();
    let n = nodes
        .iter()
        .find(|n| n.kind == NodeKind::Item && n.name == "exportedGreet");
    assert!(n.is_some(), "exported 'exportedGreet' must be extracted");
    assert_eq!(n.unwrap().item_kind, Some("function"));
}

#[test]
fn fixture_extracts_exported_class() {
    let (nodes, _) = parse_ts(REPO, FILE, &fixture_bytes()).unwrap();
    assert!(nodes.iter().any(|n| n.kind == NodeKind::Item
        && n.name == "ExportedAnimal"
        && n.item_kind == Some("class")));
}

#[test]
fn fixture_extracts_exported_interface() {
    let (nodes, _) = parse_ts(REPO, FILE, &fixture_bytes()).unwrap();
    assert!(nodes.iter().any(|n| n.kind == NodeKind::Item
        && n.name == "ExportedDescribable"
        && n.item_kind == Some("interface")));
}

#[test]
fn fixture_extracts_exported_type() {
    let (nodes, _) = parse_ts(REPO, FILE, &fixture_bytes()).unwrap();
    assert!(nodes.iter().any(|n| n.kind == NodeKind::Item
        && n.name == "ExportedPoint"
        && n.item_kind == Some("type")));
}

#[test]
fn fixture_extracts_exported_enum() {
    let (nodes, _) = parse_ts(REPO, FILE, &fixture_bytes()).unwrap();
    assert!(nodes.iter().any(|n| n.kind == NodeKind::Item
        && n.name == "ExportedColor"
        && n.item_kind == Some("enum")));
}

// ── Edges ─────────────────────────────────────────────────────────────────────

#[test]
fn fixture_declares_edges_for_all_items() {
    let (nodes, edges) = parse_ts(REPO, FILE, &fixture_bytes()).unwrap();
    let module_id = nodes
        .iter()
        .find(|n| n.kind == NodeKind::Module)
        .expect("module node must exist")
        .id
        .clone();
    for item in nodes.iter().filter(|n| n.kind == NodeKind::Item) {
        assert!(
            edges.iter().any(|e| {
                e.src == module_id && e.dst == item.id && e.kind == EdgeKind::Declares
            }),
            "missing Declares edge for '{}'",
            item.name
        );
    }
}

#[test]
fn fixture_node_ids_stable_across_reparses() {
    let src = fixture_bytes();
    let (a, _) = parse_ts(REPO, FILE, &src).unwrap();
    let (b, _) = parse_ts(REPO, FILE, &src).unwrap();
    let ids_a: Vec<_> = a.iter().map(|n| n.id.as_str()).collect();
    let ids_b: Vec<_> = b.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(ids_a, ids_b);
}

#[test]
fn fixture_all_items_have_item_kind() {
    let (nodes, _) = parse_ts(REPO, FILE, &fixture_bytes()).unwrap();
    for n in nodes.iter().filter(|n| n.kind == NodeKind::Item) {
        assert!(
            n.item_kind.is_some(),
            "item_kind missing on Item node '{}'",
            n.name
        );
    }
}

#[test]
fn fixture_all_items_have_span() {
    let (nodes, _) = parse_ts(REPO, FILE, &fixture_bytes()).unwrap();
    for n in nodes.iter().filter(|n| n.kind == NodeKind::Item) {
        assert!(n.span.is_some(), "span missing on Item node '{}'", n.name);
        let (start, end) = n.span.unwrap();
        assert!(end > start, "span end must be after start for '{}'", n.name);
    }
}
