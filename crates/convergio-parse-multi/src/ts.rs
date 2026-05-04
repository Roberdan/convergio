//! TypeScript parser: `*.ts`/`*.tsx` → `(Vec<Node>, Vec<Edge>)`.
//!
//! Uses `tree-sitter-typescript` to extract top-level declarations
//! and map them to the `convergio-graph` node taxonomy (ADR-0038 §5.3).
//!
//! # Node mapping
//!
//! | TS declaration | [`NodeKind`] | `item_kind` |
//! |---|---|---|
//! | `function_declaration` / `generator_function_declaration` | `Item` | `"function"` |
//! | `class_declaration` / `abstract_class_declaration` | `Item` | `"class"` |
//! | `interface_declaration` | `Item` | `"interface"` |
//! | `type_alias_declaration` | `Item` | `"type"` |
//! | `enum_declaration` | `Item` | `"enum"` |
//! | the file itself | `Module` | `None` |
//!
//! `export_statement` wrappers are unwrapped before classification.
//! Each item receives a [`EdgeKind::Declares`] edge from the module node.

use convergio_graph::model::{Edge, EdgeKind, Node, NodeKind};
use tree_sitter::Parser;

use crate::{
    error::{ParseError, Result},
    lang::Lang,
};

/// Parse a TypeScript source file and return graph-compatible nodes and edges.
///
/// `repo_name` is used as `crate_name` (the field is language-agnostic per
/// ADR-0038 §5.2.1 — "meaningless when repo's lang != Rust").
///
/// The returned `Vec<Node>` always contains at least one entry: the
/// `Module` node for the file. `Vec<Edge>` contains one `Declares` edge
/// per extracted item.
pub fn parse_ts(repo_name: &str, file_path: &str, source: &[u8]) -> Result<(Vec<Node>, Vec<Edge>)> {
    let mut parser = Parser::new();
    parser
        .set_language(&Lang::TypeScript.grammar())
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
        item_kind: None,
        span: None,
    };

    let mut nodes = vec![module_node];
    let mut edges = Vec::new();
    let mut cursor = root.walk();

    for child in root.children(&mut cursor) {
        let decl = unwrap_export(child);
        let Some(ik) = ts_kind_to_item_kind(decl.kind()) else {
            continue;
        };
        let Some(name) = extract_name(decl, source_str) else {
            continue;
        };

        let start = decl.start_byte() as u32;
        let end = decl.end_byte() as u32;
        let span = Some((start, end));

        let node_id = Node::compute_id(NodeKind::Item, repo_name, Some(file_path), &name, span);
        tracing::debug!(
            file = file_path,
            kind = decl.kind(),
            item_kind = ik,
            name = %name,
            "extracted ts node"
        );

        edges.push(Edge {
            src: module_id.clone(),
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
            item_kind: Some(ik),
            span,
        });
    }

    Ok((nodes, edges))
}

/// Unwrap an `export_statement` to its inner declaration node.
///
/// Returns the original node unchanged for all other node types.
fn unwrap_export(node: tree_sitter::Node<'_>) -> tree_sitter::Node<'_> {
    if node.kind() != "export_statement" {
        return node;
    }
    // Named field "declaration" covers `export function/class/interface/type/enum`.
    if let Some(decl) = node.child_by_field_name("declaration") {
        return decl;
    }
    // `export default <expr>` — field "value" or first classifiable child.
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if ts_kind_to_item_kind(child.kind()).is_some() {
            return child;
        }
    }
    node
}

/// Map a tree-sitter node type to a graph `item_kind` string.
///
/// Returns `None` for node types that are not top-level declarations
/// we want to index (e.g. imports, comments, expressions).
fn ts_kind_to_item_kind(kind: &str) -> Option<&'static str> {
    match kind {
        "function_declaration" | "generator_function_declaration" => Some("function"),
        "class_declaration" | "abstract_class_declaration" => Some("class"),
        "interface_declaration" => Some("interface"),
        "type_alias_declaration" => Some("type"),
        "enum_declaration" => Some("enum"),
        _ => None,
    }
}

/// Extract an identifier name from a tree-sitter declaration node.
///
/// Tries the named field `"name"` first (works for most TS declarations),
/// then falls back to scanning for the first `identifier` or
/// `type_identifier` child.
fn extract_name(node: tree_sitter::Node<'_>, source: &str) -> Option<String> {
    if let Some(name_node) = node.child_by_field_name("name") {
        return source
            .get(name_node.start_byte()..name_node.end_byte())
            .map(str::to_owned);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" || child.kind() == "type_identifier" {
            return source
                .get(child.start_byte()..child.end_byte())
                .map(str::to_owned);
        }
    }
    None
}
