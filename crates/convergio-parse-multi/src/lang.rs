//! Language discriminant and grammar resolution.

use tree_sitter::Language;

/// Supported source languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lang {
    /// TypeScript (`.ts`, `.tsx`).
    TypeScript,
    /// Python (`.py`).
    Python,
}

impl Lang {
    /// Return the tree-sitter [`Language`] for this grammar.
    #[must_use]
    pub fn grammar(self) -> Language {
        match self {
            Lang::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Lang::Python => tree_sitter_python::LANGUAGE.into(),
        }
    }

    /// Infer language from a file extension. Returns `None` if unknown.
    #[must_use]
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "ts" | "tsx" => Some(Lang::TypeScript),
            "py" => Some(Lang::Python),
            _ => None,
        }
    }

    /// Canonical string label (used as graph `crate_name` analogue for
    /// non-Rust repos).
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Lang::TypeScript => "typescript",
            Lang::Python => "python",
        }
    }
}

impl std::fmt::Display for Lang {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_extension_roundtrip() {
        assert_eq!(Lang::from_extension("ts"), Some(Lang::TypeScript));
        assert_eq!(Lang::from_extension("tsx"), Some(Lang::TypeScript));
        assert_eq!(Lang::from_extension("py"), Some(Lang::Python));
        assert_eq!(Lang::from_extension("rs"), None);
    }

    #[test]
    fn label_non_empty() {
        assert!(!Lang::TypeScript.label().is_empty());
        assert!(!Lang::Python.label().is_empty());
    }

    #[test]
    fn grammar_loads() {
        let _ = Lang::TypeScript.grammar();
        let _ = Lang::Python.grammar();
    }
}
