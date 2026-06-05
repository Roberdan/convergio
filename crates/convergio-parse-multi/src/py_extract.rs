//! Python helpers extracted from `py.rs` to keep `py.rs` well under
//! the 300-line cap (audit follow-up, 2026-05-12 — see
//! `docs/reviews/crate-audits/convergio-parse-multi.md`).
//!
//! Owns:
//! - method extraction for class bodies ([`extract_methods`])
//! - docstring extraction ([`extract_docstring`])
//! - small AST helpers ([`unwrap_decorated`], [`py_kind_to_item_kind`],
//!   [`py_extract_name`])
//! - skip filter for `__pycache__` / `.venv` paths ([`should_skip`])
//!
//! These were `py.rs`-private helpers; promoting them keeps the public
//! surface of the crate identical (only `parse_py` is re-exported) while
//! giving us room to add behavior without immediately tripping the cap.

use convergio_graph::model::{Edge, EdgeKind, Node, NodeKind};

/// Walk a class body and append a `method` `Item` node per
/// `function_definition` child, with a `Declares` edge from `class_id`.
pub(crate) fn extract_methods(
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

/// Return `true` for paths that should not be parsed
/// (`__pycache__/`, `.venv/`).
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

/// Unwrap a `decorated_definition` to its inner `function_definition` or
/// `class_definition`. Returns the input unchanged for other node types.
pub(crate) fn unwrap_decorated(node: tree_sitter::Node<'_>) -> tree_sitter::Node<'_> {
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

/// Map a tree-sitter Python node kind to a graph `item_kind` string.
pub(crate) fn py_kind_to_item_kind(kind: &str) -> Option<&'static str> {
    match kind {
        "function_definition" => Some("function"),
        "class_definition" => Some("class"),
        _ => None,
    }
}

/// Extract the identifier name from a Python declaration node.
pub(crate) fn py_extract_name(node: tree_sitter::Node<'_>, source: &str) -> Option<String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skip_filters_pycache_and_venv() {
        assert!(should_skip("project/__pycache__/foo.py"));
        assert!(should_skip(".venv/lib/foo.py"));
        assert!(!should_skip("src/foo.py"));
    }

    #[test]
    fn py_kind_to_item_kind_known_kinds() {
        assert_eq!(
            py_kind_to_item_kind("function_definition"),
            Some("function")
        );
        assert_eq!(py_kind_to_item_kind("class_definition"), Some("class"));
        assert_eq!(py_kind_to_item_kind("import_statement"), None);
    }
}
