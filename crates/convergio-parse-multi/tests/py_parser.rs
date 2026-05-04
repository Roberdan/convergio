//! Integration tests for the Python parser (F2-3).
//!
//! Each test loads `tests/fixtures/py/sample.py` and asserts the
//! `(Vec<Node>, Vec<Edge>)` output against known declarations in that file.
//! Using a file fixture (rather than inline bytes) validates the full
//! read-from-disk path used by the fleet graph builder.

use convergio_graph::model::{EdgeKind, NodeKind};
use convergio_parse_multi::{parse_py, py::extract_docstring, py::should_skip, Lang};

fn fixture_bytes() -> Vec<u8> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/py/sample.py");
    std::fs::read(path).expect("fixture file must exist at tests/fixtures/py/sample.py")
}

const REPO: &str = "test-repo";
const FILE: &str = "tests/fixtures/py/sample.py";

// ── Module node ───────────────────────────────────────────────────────────────

#[test]
fn fixture_has_module_node() {
    let (nodes, _) = parse_py(REPO, FILE, &fixture_bytes()).unwrap();
    let module = nodes.iter().find(|n| n.kind == NodeKind::Module);
    assert!(module.is_some(), "module node must be present");
    assert_eq!(module.unwrap().crate_name, REPO);
    assert_eq!(module.unwrap().file_path.as_deref(), Some(FILE));
}

// ── Item nodes ────────────────────────────────────────────────────────────────

#[test]
fn fixture_extracts_function_greet() {
    let (nodes, _) = parse_py(REPO, FILE, &fixture_bytes()).unwrap();
    let n = nodes
        .iter()
        .find(|n| n.kind == NodeKind::Item && n.name == "greet");
    assert!(n.is_some(), "'greet' must be extracted");
    assert_eq!(n.unwrap().item_kind, Some("function"));
}

#[test]
fn fixture_extracts_class_animal() {
    let (nodes, _) = parse_py(REPO, FILE, &fixture_bytes()).unwrap();
    let n = nodes
        .iter()
        .find(|n| n.kind == NodeKind::Item && n.name == "Animal");
    assert!(n.is_some(), "'Animal' must be extracted");
    assert_eq!(n.unwrap().item_kind, Some("class"));
}

#[test]
fn fixture_extracts_class_dog() {
    let (nodes, _) = parse_py(REPO, FILE, &fixture_bytes()).unwrap();
    let n = nodes
        .iter()
        .find(|n| n.kind == NodeKind::Item && n.name == "Dog");
    assert!(n.is_some(), "'Dog' must be extracted");
    assert_eq!(n.unwrap().item_kind, Some("class"));
}

#[test]
fn fixture_extracts_decorated_standalone() {
    let (nodes, _) = parse_py(REPO, FILE, &fixture_bytes()).unwrap();
    let n = nodes
        .iter()
        .find(|n| n.kind == NodeKind::Item && n.name == "standalone");
    assert!(n.is_some(), "'standalone' (decorated) must be extracted");
    assert_eq!(n.unwrap().item_kind, Some("function"));
}

// ── Methods ───────────────────────────────────────────────────────────────────

#[test]
fn fixture_extracts_methods_under_animal() {
    let (nodes, edges) = parse_py(REPO, FILE, &fixture_bytes()).unwrap();
    let cls = nodes
        .iter()
        .find(|n| n.kind == NodeKind::Item && n.name == "Animal")
        .expect("Animal class must exist");
    let methods: Vec<_> = nodes
        .iter()
        .filter(|n| n.item_kind == Some("method"))
        .collect();
    assert!(!methods.is_empty(), "no method nodes found");
    assert!(
        methods.iter().any(|m| m.name == "__init__"),
        "__init__ method missing"
    );
    assert!(
        methods.iter().any(|m| m.name == "speak"),
        "speak method missing"
    );
    for m in &methods {
        if m.name == "__init__" || m.name == "speak" {
            assert!(
                edges.iter().any(|e| e.src == cls.id && e.dst == m.id),
                "Declares edge from Animal to {} missing",
                m.name
            );
        }
    }
}

#[test]
fn fixture_extracts_bark_under_dog() {
    let (nodes, edges) = parse_py(REPO, FILE, &fixture_bytes()).unwrap();
    let dog = nodes
        .iter()
        .find(|n| n.kind == NodeKind::Item && n.name == "Dog")
        .expect("Dog class must exist");
    let bark = nodes
        .iter()
        .find(|n| n.item_kind == Some("method") && n.name == "bark");
    assert!(bark.is_some(), "bark method missing");
    assert!(
        edges
            .iter()
            .any(|e| e.src == dog.id && e.dst == bark.unwrap().id),
        "Declares edge from Dog to bark missing"
    );
}

// ── Edges ─────────────────────────────────────────────────────────────────────

#[test]
fn fixture_declares_edges_for_top_level_items() {
    let (nodes, edges) = parse_py(REPO, FILE, &fixture_bytes()).unwrap();
    let module = nodes
        .iter()
        .find(|n| n.kind == NodeKind::Module)
        .expect("module node must exist");
    let top_level: Vec<_> = nodes
        .iter()
        .filter(|n| n.item_kind == Some("function") || n.item_kind == Some("class"))
        .collect();
    for item in top_level {
        assert!(
            edges
                .iter()
                .any(|e| e.src == module.id && e.dst == item.id && e.kind == EdgeKind::Declares),
            "Declares edge from module to '{}' missing",
            item.name
        );
    }
}

#[test]
fn fixture_all_items_have_span() {
    let (nodes, _) = parse_py(REPO, FILE, &fixture_bytes()).unwrap();
    for n in nodes.iter().filter(|n| n.kind == NodeKind::Item) {
        assert!(n.span.is_some(), "item '{}' has no span", n.name);
    }
}

#[test]
fn fixture_all_items_have_item_kind() {
    let (nodes, _) = parse_py(REPO, FILE, &fixture_bytes()).unwrap();
    for n in nodes.iter().filter(|n| n.kind == NodeKind::Item) {
        assert!(n.item_kind.is_some(), "item '{}' has no item_kind", n.name);
    }
}

#[test]
fn fixture_node_ids_stable_across_reparses() {
    let bytes = fixture_bytes();
    let (nodes1, _) = parse_py(REPO, FILE, &bytes).unwrap();
    let (nodes2, _) = parse_py(REPO, FILE, &bytes).unwrap();
    assert_eq!(nodes1.len(), nodes2.len());
    for (a, b) in nodes1.iter().zip(nodes2.iter()) {
        assert_eq!(a.id, b.id, "node id changed between parses: {}", a.name);
    }
}

// ── Docstring ─────────────────────────────────────────────────────────────────

#[test]
fn fixture_greet_has_docstring() {
    let src = fixture_bytes();
    let source_str = std::str::from_utf8(&src).unwrap();
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&Lang::Python.grammar())
        .expect("grammar load");
    let tree = parser.parse(&src, None).unwrap();
    let root = tree.root_node();
    let mut cursor = root.walk();
    let greet_node = root
        .children(&mut cursor)
        .find(|n| n.kind() == "function_definition")
        .expect("greet function_definition not found");
    let doc = extract_docstring(greet_node, source_str);
    assert!(doc.is_some(), "greet docstring must be captured");
    assert!(
        doc.unwrap().contains("greeting"),
        "docstring should contain 'greeting'"
    );
}

// ── Skip patterns ─────────────────────────────────────────────────────────────

#[test]
fn pycache_path_skipped() {
    let src = b"def foo(): pass\n";
    let (nodes, edges) = parse_py(REPO, "__pycache__/mod.cpython-311.pyc", src).unwrap();
    assert!(
        nodes.is_empty() && edges.is_empty(),
        "__pycache__ path must yield empty result"
    );
}

#[test]
fn venv_path_skipped() {
    let src = b"def bar(): pass\n";
    let (nodes, edges) = parse_py(REPO, ".venv/lib/python3.11/site-packages/foo.py", src).unwrap();
    assert!(
        nodes.is_empty() && edges.is_empty(),
        ".venv path must yield empty result"
    );
}

#[test]
fn should_skip_helper() {
    assert!(should_skip("__pycache__/foo.pyc"));
    assert!(should_skip(".venv/lib/foo.py"));
    assert!(!should_skip("src/main.py"));
}
