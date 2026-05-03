//! Selective-embedding policy.
//!
//! ADR-0035 § 5.4: we do **not** embed every parsed node. The policy
//! lives here, separate from storage and inference, so it can be
//! reviewed and tuned in isolation.

/// Categories of node a fleet caller may produce. Callers map their
/// own taxonomy (`convergio-graph::NodeKind`, a TS tree-sitter kind,
/// a Python AST node, …) into one of these values when asking the
/// policy. This decouples `convergio-embed` from any specific
/// language parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EmbedTarget {
    /// Crate root or top-level package.
    Crate,
    /// Module / namespace / file-level scope.
    Module,
    /// Function / class / struct / interface that has a docstring or
    /// `///` doc comment.
    DocumentedItem,
    /// Function / class / struct / interface without any
    /// documentation. Skipped by the default policy because the
    /// signal-to-noise ratio of the embeddable text is too low.
    UndocumentedItem,
    /// Architecture Decision Record (Markdown).
    Adr,
    /// Other prose document (README, spec, runbook).
    Doc,
}

/// Default selective-embedding policy: ADR-0035 § 5.4 verbatim.
///
/// Embed crates, modules, documented items, ADRs, prose docs.
/// Skip undocumented private items.
#[derive(Debug, Clone, Copy, Default)]
pub struct EmbedPolicy;

impl EmbedPolicy {
    /// Decide whether to embed a node of the given target category.
    pub fn should_embed(self, target: EmbedTarget) -> bool {
        match target {
            EmbedTarget::Crate
            | EmbedTarget::Module
            | EmbedTarget::DocumentedItem
            | EmbedTarget::Adr
            | EmbedTarget::Doc => true,
            EmbedTarget::UndocumentedItem => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{EmbedPolicy, EmbedTarget};

    #[test]
    fn embeds_crates_modules_documented_items_adrs_docs() {
        let p = EmbedPolicy;
        assert!(p.should_embed(EmbedTarget::Crate));
        assert!(p.should_embed(EmbedTarget::Module));
        assert!(p.should_embed(EmbedTarget::DocumentedItem));
        assert!(p.should_embed(EmbedTarget::Adr));
        assert!(p.should_embed(EmbedTarget::Doc));
    }

    #[test]
    fn skips_undocumented_items() {
        assert!(!EmbedPolicy.should_embed(EmbedTarget::UndocumentedItem));
    }
}
