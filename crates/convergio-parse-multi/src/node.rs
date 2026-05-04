//! Parsed node representation shared across languages.

/// Coarse node kind mapped from tree-sitter node types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    /// Top-level function declaration.
    Function,
    /// Class declaration.
    Class,
    /// Interface (TypeScript) or abstract base class (Python stub).
    Interface,
    /// Module-level variable or constant binding.
    Variable,
    /// Any node kind not mapped above.
    Other(String),
}

impl NodeKind {
    /// Build from a tree-sitter node type string.
    #[must_use]
    pub fn from_ts_kind(kind: &str) -> Self {
        match kind {
            "function_declaration"
            | "function_definition"
            | "method_definition"
            | "arrow_function" => NodeKind::Function,
            "class_declaration" | "class_definition" => NodeKind::Class,
            "interface_declaration" => NodeKind::Interface,
            "variable_declaration" | "lexical_declaration" | "expression_statement" => {
                NodeKind::Variable
            }
            other => NodeKind::Other(other.to_owned()),
        }
    }
}

/// A single AST node extracted from a source file.
#[derive(Debug, Clone)]
pub struct ParsedNode {
    /// Identifier name, if the node has one.
    pub name: Option<String>,
    /// Coarse kind classification.
    pub kind: NodeKind,
    /// Byte span `[start, end)` in the source buffer.
    pub span: (u32, u32),
    /// 1-based start row in the source file.
    pub row: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_from_ts_kind_function() {
        assert_eq!(
            NodeKind::from_ts_kind("function_declaration"),
            NodeKind::Function
        );
        assert_eq!(
            NodeKind::from_ts_kind("function_definition"),
            NodeKind::Function
        );
    }

    #[test]
    fn kind_from_ts_kind_unknown() {
        assert_eq!(
            NodeKind::from_ts_kind("import_statement"),
            NodeKind::Other("import_statement".to_owned())
        );
    }
}
