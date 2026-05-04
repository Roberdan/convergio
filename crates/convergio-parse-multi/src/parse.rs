//! Core parse routine: bytes → `Vec<ParsedNode>`.

use tree_sitter::Parser;

use crate::{
    error::{ParseError, Result},
    lang::Lang,
    node::{NodeKind, ParsedNode},
};

/// Parse `source` bytes as `lang` and return top-level AST nodes.
///
/// Only direct children of the root node are returned; the caller
/// decides how deep to recurse for fleet-graph ingestion.
pub fn parse(lang: Lang, source: &[u8], file: &str) -> Result<Vec<ParsedNode>> {
    let mut parser = Parser::new();
    parser
        .set_language(&lang.grammar())
        .expect("grammar version mismatch — rebuild required");

    let tree = parser
        .parse(source, None)
        .ok_or_else(|| ParseError::ParserFailed {
            file: file.to_owned(),
        })?;

    let root = tree.root_node();
    if root.has_error() {
        tracing::warn!(file, "tree-sitter reported syntax errors; continuing");
    }

    let source_str = std::str::from_utf8(source).map_err(|e| ParseError::Encoding {
        file: file.to_owned(),
        source: e,
    })?;

    let mut nodes = Vec::new();
    let mut cursor = root.walk();

    for child in root.children(&mut cursor) {
        let kind = NodeKind::from_ts_kind(child.kind());
        let name = extract_name(&child, source_str);

        let start = child.start_byte() as u32;
        let end = child.end_byte() as u32;
        let row = child.start_position().row as u32 + 1;

        tracing::debug!(file, kind = child.kind(), ?name, row, "extracted node");

        nodes.push(ParsedNode {
            name,
            kind,
            span: (start, end),
            row,
        });
    }

    Ok(nodes)
}

/// Try to extract the identifier name from the first `identifier` child.
fn extract_name(node: &tree_sitter::Node<'_>, source: &str) -> Option<String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_typescript_function() {
        let src = b"function hello() { return 42; }";
        let nodes = parse(Lang::TypeScript, src, "test.ts").unwrap();
        assert!(!nodes.is_empty());
        let fn_node = nodes.iter().find(|n| n.kind == NodeKind::Function);
        assert!(fn_node.is_some());
        assert_eq!(fn_node.unwrap().name.as_deref(), Some("hello"));
    }

    #[test]
    fn parse_python_function() {
        let src = b"def greet():\n    pass\n";
        let nodes = parse(Lang::Python, src, "test.py").unwrap();
        assert!(!nodes.is_empty());
        let fn_node = nodes.iter().find(|n| n.kind == NodeKind::Function);
        assert!(fn_node.is_some());
        assert_eq!(fn_node.unwrap().name.as_deref(), Some("greet"));
    }
}
