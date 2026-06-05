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
//! captured via [`crate::py_extract::extract_docstring`]. Currently used
//! for tracing; callers may invoke it directly for indexing.
//!
//! # Skip patterns
//!
//! Files whose path contains `__pycache__` or `.venv` return
//! `Ok((vec![], vec![]))` immediately so the fleet walker can filter
//! cheaply via [`crate::py_extract::should_skip`].
//!
//! # Module split
//!
//! Helpers (`extract_methods`, `extract_docstring`, `unwrap_decorated`,
//! `py_kind_to_item_kind`, `py_extract_name`, `should_skip`) live in
//! `py_extract.rs` to keep this file under the 300-line cap; see the
//! 2026-05-12 audit follow-up.

use convergio_graph::model::{Edge, EdgeKind, Node, NodeKind};
use tree_sitter::Parser;

use crate::{
    error::{ParseError, Result},
    lang::Lang,
    py_extract::{extract_methods, py_extract_name, py_kind_to_item_kind, unwrap_decorated},
};

#[doc(inline)]
pub use crate::py_extract::{extract_docstring, should_skip};

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
///
/// **Partial-parse semantics**: when tree-sitter reports
/// `root.has_error()` this function logs `warn!` and still returns
/// the extractable nodes/edges. See [`ParseError`](crate::ParseError)
/// for the rationale.
pub fn parse_py(repo_name: &str, file_path: &str, source: &[u8]) -> Result<(Vec<Node>, Vec<Edge>)> {
    if should_skip(file_path) {
        tracing::debug!(file = file_path, "skipping __pycache__/.venv path");
        return Ok((vec![], vec![]));
    }

    let mut parser = Parser::new();
    parser
        .set_language(&Lang::Python.grammar())
        .map_err(|source| ParseError::GrammarVersionMismatch {
            lang: Lang::Python.label(),
            source,
        })?;

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
