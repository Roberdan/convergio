//! Domain types for the code-graph layer (ADR-0014).

use serde::{Deserialize, Serialize};

/// What kind of thing a graph node represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    /// A workspace member crate.
    Crate,
    /// A Rust module within a crate (file or `mod { }` block).
    Module,
    /// A code item (struct / enum / fn / trait / impl / const / type / macro).
    Item,
    /// An ADR document.
    Adr,
    /// A non-ADR markdown doc (README, plans, etc.).
    Doc,
}

impl NodeKind {
    /// String tag persisted in `graph_nodes.kind`.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Crate => "crate",
            Self::Module => "module",
            Self::Item => "item",
            Self::Adr => "adr",
            Self::Doc => "doc",
        }
    }
}

/// What kind of relationship an edge represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    /// `src` references `dst` (a `use` path or call site).
    Uses,
    /// `src` declares `dst` (a module declares its items, a crate
    /// declares its modules).
    Declares,
    /// `src` re-exports `dst` (a `pub use` path).
    ReExports,
    /// `src` (an ADR) claims to touch crate `dst`
    /// (frontmatter `touches_crates`).
    Claims,
    /// `src` (a doc) mentions `dst` (a code symbol or another doc).
    Mentions,
    /// `src` (a crate) depends on `dst` (another crate) via Cargo.
    DependsOn,
    /// Cross-repo semantic similarity (cosine ≥ 0.85, ADR-0038).
    SimilarTo,
    /// Cross-repo structural duplicate (cosine ≥ 0.95 + same node kind, ADR-0038).
    Duplicates,
}

impl EdgeKind {
    /// String tag persisted in `graph_edges.kind`.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Uses => "uses",
            Self::Declares => "declares",
            Self::ReExports => "re_exports",
            Self::Claims => "claims",
            Self::Mentions => "mentions",
            Self::DependsOn => "depends_on",
            Self::SimilarTo => "similar_to",
            Self::Duplicates => "duplicates",
        }
    }
}

/// A graph node — code or doc.
///
/// `id` is a stable hash of the node's identity, computed by
/// [`Node::compute_id`]. Two parses of the same crate against the
/// same file produce the same id, which lets the parser do upserts
/// without growing the row count.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Node {
    /// Stable identity hash (sha256, hex, first 16 chars).
    pub id: String,
    /// What this node represents.
    pub kind: NodeKind,
    /// Display name (`convergio-cli`, `commands::session`, `Brief`, ...).
    pub name: String,
    /// Path to the source file (None for ADR/doc-only nodes).
    pub file_path: Option<String>,
    /// Owning crate name. `__docs__` for non-code nodes.
    pub crate_name: String,
    /// Owning repository (e.g. `convergio`). Enables fleet multi-repo graphs.
    pub repo: String,
    /// For `kind == Item`, the item flavour (struct/fn/...).
    pub item_kind: Option<&'static str>,
    /// Byte offset span in `file_path` (None for non-code).
    pub span: Option<(u32, u32)>,
}

/// Sentinel crate name for nodes that do not belong to a code crate.
pub const DOCS_CRATE: &str = "__docs__";

impl Node {
    /// Compute a stable hash from the node identity.
    ///
    /// The hash inputs are: `kind`, `repo`, `crate_name`, `file_path`
    /// (if any), `name`, and `span` (if any). Two nodes with the same
    /// inputs collapse to the same `id`.
    pub fn compute_id(
        kind: NodeKind,
        repo: &str,
        crate_name: &str,
        file_path: Option<&str>,
        name: &str,
        span: Option<(u32, u32)>,
    ) -> String {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(kind.as_str().as_bytes());
        h.update(b"|");
        h.update(repo.as_bytes());
        h.update(b"|");
        h.update(crate_name.as_bytes());
        h.update(b"|");
        h.update(file_path.unwrap_or("").as_bytes());
        h.update(b"|");
        h.update(name.as_bytes());
        if let Some((s, e)) = span {
            h.update(b"|");
            h.update(s.to_le_bytes());
            h.update(e.to_le_bytes());
        }
        let digest = h.finalize();
        hex::encode(&digest[..8])
    }
}

/// A directed edge between two nodes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Edge {
    /// Source node id.
    pub src: String,
    /// Destination node id.
    pub dst: String,
    /// Relationship kind.
    pub kind: EdgeKind,
    /// Multiplicity hint (e.g. number of call sites).
    pub weight: u32,
}

/// Aggregate result of a build pass — useful for tests and CLI output.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BuildReport {
    /// Number of nodes after the pass.
    pub nodes: usize,
    /// Number of edges after the pass.
    pub edges: usize,
    /// Number of crates discovered via `cargo metadata`.
    pub crates: usize,
    /// Number of `*.rs` files parsed.
    pub files_parsed: usize,
    /// Number of files skipped because their mtime equals the stored value.
    pub files_skipped: usize,
    /// Number of doc-link edges (claims/mentions from ADRs and docs)
    /// that could not be inserted because their destination node did
    /// not exist (e.g. an ADR mentions a crate that no longer ships,
    /// or a related-ADR id that was never written).
    #[serde(default)]
    pub doc_edges_skipped: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_id_is_stable() {
        let a = Node::compute_id(
            NodeKind::Item,
            "convergio",
            "convergio-cli",
            Some("src/lib.rs"),
            "Brief",
            None,
        );
        let b = Node::compute_id(
            NodeKind::Item,
            "convergio",
            "convergio-cli",
            Some("src/lib.rs"),
            "Brief",
            None,
        );
        assert_eq!(a, b);
    }

    #[test]
    fn node_id_changes_with_inputs() {
        let a = Node::compute_id(
            NodeKind::Item,
            "convergio",
            "convergio-cli",
            Some("src/lib.rs"),
            "Brief",
            None,
        );
        let b = Node::compute_id(
            NodeKind::Item,
            "convergio",
            "convergio-cli",
            Some("src/lib.rs"),
            "Other",
            None,
        );
        assert_ne!(a, b);
    }

    #[test]
    fn node_id_changes_with_repo() {
        let a = Node::compute_id(
            NodeKind::Item,
            "convergio",
            "convergio-cli",
            Some("src/lib.rs"),
            "Brief",
            None,
        );
        let b = Node::compute_id(
            NodeKind::Item,
            "other-repo",
            "convergio-cli",
            Some("src/lib.rs"),
            "Brief",
            None,
        );
        assert_ne!(a, b);
    }

    #[test]
    fn kind_round_trips_via_str() {
        assert_eq!(NodeKind::Crate.as_str(), "crate");
        assert_eq!(NodeKind::Item.as_str(), "item");
        assert_eq!(EdgeKind::DependsOn.as_str(), "depends_on");
    }
}
