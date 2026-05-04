//! Python parser: `*.py` → `(Vec<Node>, Vec<Edge>)`.
//!
//! Uses `tree-sitter-python` to extract top-level declarations
//! and map them to the `convergio-graph` node taxonomy (ADR-0038 §5.3).
//!
//! # Node mapping
//!
//! | Python construct | [`NodeKind`] | `item_kind` |
//! |---|---|---|
//! | `function_definition` (top-level) | `Item` | `"function"` |
//! | `class_definition` (top-level) | `Item` | `"class"` |
//! | `function_definition` inside class body | `Item` | `"method"` |
//! | the file itself | `Module` | `None` |
//!
//! `decorated_definition` wrappers are unwrapped before classification.
//! Each item receives a [`EdgeKind::Declares`] edge from its parent
//! (module for top-level items, class node for methods).
//!
//! # Docstring extraction
//!
//! If the first non-comment statement in a function or class body is a
//! bare string literal (`expression_statement` → `string`), it is
//! captured via [`extract_docstring`]. Currently used for tracing; callers
//! may invoke it directly for indexing.
//!
//! # Skip patterns
//!
//! Files whose path contains `__pycache__` or `.venv` return
//! `Ok((vec![], vec![]))` immediately so the fleet walker can filter cheaply.

use convergio_graph::model::{Edge, EdgeKind, Node, NodeKind};
use tree_sitter::Parser;

use crate::{
    error::{ParseError, Result},
    lang::Lang,
};

/// Parse a Python source file and return graph-compatible nodes and edges.
///
/// `repo_name` is used as `crate_name` (the field is language-agnostic per
/// ADR-0038 §5.2.1 — "meaningless when repo's lang != Rust").
///
/// Files under `__pycache__` or `.venv` return empty vecs without parsing.
///
/// The returned `Vec<Node>` always contains at least one entry (the `Module`
/// node) for non-skipped files. `Vec<Edge>` contains one `Declares` edge
/// per extracted item.
pub fn parse_py(repo_name: &str, file_path: &str, source: &[u8]) -> Result<(Vec<Node>, Vec<Edge>)> {
    if should_skip(file_path) {
        tracing::debug!(file = file_path, "skipping __pycache__/.venv path");
        return Ok((vec![], vec![]));
    }

    let mut parser = Parser::new();
    parser
        .set_language(&Lang::Python.grammar())
        .expect("grammar version mismatch — rebuild required");

    let tree = parser
        .parse(source, None)
        .ok_or_else(|| ParseError::ParserFailed {
            file: file_path.to_owned(),
        })?;

    let root = tree.root_node();
    if root.has_error() {
        tracing::warn!(
            file = file_path,
            "tree-sitter reported syntax errors; continuing"
        );
    }

    let source_str = std::str::from_utf8(source).map_err(|e| ParseError::Encoding {
        file: file_path.to_owned(),
        source: e,
    })?;

    let module_id = Node::compute_id(
        NodeKind::Module,
        repo_name,
        repo_name,
        Some(file_path),
        file_path,
        None,
    );
    let module_node = Node {
        id: module_id.clone(),
        kind: NodeKind::Module,
        name: file_path.to_owned(),
        file_path: Some(file_path.to_owned()),
        crate_name: repo_name.to_owned(),
        repo: repo_name.to_owned(),
        item_kind: None,
        span: None,
    };

    let mut nodes = vec![module_node];
    let mut edges = Vec::new();
    let mut cursor = root.walk();

    for child in root.children(&mut cursor) {
        let decl = unwrap_decorated(child);
        let Some(ik) = py_kind_to_item_kind(decl.kind()) else {
            continue;
        };
        let Some(name) = py_extract_name(decl, source_str) else {
            continue;
        };

        let start = decl.start_byte() as u32;
        let end = decl.end_byte() as u32;
        let span = Some((start, end));

        let node_id = Node::compute_id(
            NodeKind::Item,
            repo_name,
            repo_name,
            Some(file_path),
            &name,
            span,
        );
        tracing::debug!(
            file = file_path,
            kind = decl.kind(),
            item_kind = ik,
            name = %name,
            docstring = extract_docstring(decl, source_str).as_deref().unwrap_or(""),
            "extracted py node"
        );

        edges.push(Edge {
            src: module_id.clone(),
            dst: node_id.clone(),
            kind: EdgeKind::Declares,
            weight: 1,
        });
        nodes.push(Node {
            id: node_id.clone(),
            kind: NodeKind::Item,
            name: name.clone(),
            file_path: Some(file_path.to_owned()),
            crate_name: repo_name.to_owned(),
            repo: repo_name.to_owned(),
            item_kind: Some(ik),
            span,
        });

        if ik == "class" {
            extract_methods(
                decl, source_str, repo_name, file_path, &node_id, &mut nodes, &mut edges,
            );
        }
    }

    Ok((nodes, edges))
}

fn extract_methods(
    class_node: tree_sitter::Node<'_>,
    source: &str,
    repo_name: &str,
    file_path: &str,
    class_id: &str,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
) {
    let Some(body) = class_node.child_by_field_name("body") else {
        return;
    };
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        let decl = unwrap_decorated(child);
        if decl.kind() != "function_definition" {
            continue;
        }
        let Some(name) = py_extract_name(decl, source) else {
            continue;
        };

        let start = decl.start_byte() as u32;
        let end = decl.end_byte() as u32;
        let span = Some((start, end));
        let node_id = Node::compute_id(
            NodeKind::Item,
            repo_name,
            repo_name,
            Some(file_path),
            &name,
            span,
        );

        tracing::debug!(
            file = file_path,
            item_kind = "method",
            name = %name,
            docstring = extract_docstring(decl, source).as_deref().unwrap_or(""),
            "extracted py method"
        );

        edges.push(Edge {
            src: class_id.to_owned(),
            dst: node_id.clone(),
            kind: EdgeKind::Declares,
            weight: 1,
        });
        nodes.push(Node {
            id: node_id,
            kind: NodeKind::Item,
            name,
            file_path: Some(file_path.to_owned()),
            crate_name: repo_name.to_owned(),
            repo: repo_name.to_owned(),
            item_kind: Some("method"),
            span,
        });
    }
}

/// Return `true` for paths that should not be parsed.
pub fn should_skip(file_path: &str) -> bool {
    file_path.contains("__pycache__") || file_path.contains(".venv")
}

/// Extract the docstring from a function or class body, if present.
///
/// Returns the raw string token (including quotes) of the first
/// `expression_statement → string` in the body, skipping leading comments.
pub fn extract_docstring(node: tree_sitter::Node<'_>, source: &str) -> Option<String> {
    let body = node.child_by_field_name("body")?;
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        match child.kind() {
            "comment" => continue,
            "expression_statement" => {
                let mut expr_cursor = child.walk();
                for expr in child.children(&mut expr_cursor) {
                    if expr.kind() == "string" {
                        return source
                            .get(expr.start_byte()..expr.end_byte())
                            .map(str::to_owned);
                    }
                }
                return None;
            }
            _ => return None,
        }
    }
    None
}

fn unwrap_decorated(node: tree_sitter::Node<'_>) -> tree_sitter::Node<'_> {
    if node.kind() != "decorated_definition" {
        return node;
    }
    if let Some(def) = node.child_by_field_name("definition") {
        return def;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if py_kind_to_item_kind(child.kind()).is_some() {
            return child;
        }
    }
    node
}

fn py_kind_to_item_kind(kind: &str) -> Option<&'static str> {
    match kind {
        "function_definition" => Some("function"),
        "class_definition" => Some("class"),
        _ => None,
    }
}

fn py_extract_name(node: tree_sitter::Node<'_>, source: &str) -> Option<String> {
    if let Some(name_node) = node.child_by_field_name("name") {
        return source
            .get(name_node.start_byte()..name_node.end_byte())
            .map(str::to_owned);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            return source
                .get(child.start_byte()..child.end_byte())
                .map(str::to_owned);
        }
    }
    None
}
