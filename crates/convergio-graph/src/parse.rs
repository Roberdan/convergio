//! syn-based parser: walks a single Rust source file and emits the
//! nodes + edges it contains.
//!
//! v0 scope (ADR-0014): no name resolution, no type resolution, no
//! macro expansion. We capture what is *written*, not what it means.

use crate::error::{GraphError, Result};
use crate::model::{Edge, EdgeKind, Node, NodeKind};
use crate::parse_use::{flatten_use, path_to_string};
use std::path::Path;
use syn::visit::Visit;

/// Parse one `*.rs` file into nodes + edges scoped to a single
/// module within a crate.
///
/// `repo` is the owning repository name (e.g. `convergio`).
/// `crate_name` is the owning workspace crate (e.g. `convergio-cli`).
/// `module_path` is the dotted path inside that crate
/// (e.g. `commands::session`); empty string for the crate root.
/// `abs_path` is read from disk; `rel_path` is what gets stamped on
/// every produced node and folded into the stable id (so the graph
/// is portable across clones at different filesystem paths).
pub fn parse_file(
    repo: &str,
    crate_name: &str,
    module_path: &str,
    abs_path: &Path,
    rel_path: &str,
) -> Result<(Vec<Node>, Vec<Edge>)> {
    let source = std::fs::read_to_string(abs_path)?;
    let parsed = syn::parse_file(&source).map_err(|err| GraphError::Syn {
        file: abs_path.display().to_string(),
        err,
    })?;

    let module_name = if module_path.is_empty() {
        "<root>".to_string()
    } else {
        module_path.to_string()
    };
    let module_id = Node::compute_id(
        NodeKind::Module,
        repo,
        crate_name,
        Some(rel_path),
        &module_name,
        None,
    );

    let mut visitor = ItemVisitor {
        repo: repo.to_string(),
        crate_name: crate_name.to_string(),
        module_path: module_path.to_string(),
        file_path: rel_path.to_string(),
        module_id: module_id.clone(),
        nodes: Vec::new(),
        edges: Vec::new(),
    };
    let module_node = Node {
        id: module_id,
        kind: NodeKind::Module,
        name: module_name,
        file_path: Some(visitor.file_path.clone()),
        crate_name: crate_name.to_string(),
        repo: repo.to_string(),
        item_kind: None,
        span: None,
    };
    visitor.nodes.push(module_node);
    visitor.visit_file(&parsed);
    Ok((visitor.nodes, visitor.edges))
}

struct ItemVisitor {
    repo: String,
    crate_name: String,
    #[allow(dead_code)] // reserved for future nested-module path tracking
    module_path: String,
    file_path: String,
    module_id: String,
    nodes: Vec<Node>,
    edges: Vec<Edge>,
}

impl ItemVisitor {
    fn record_item(&mut self, name: &str, item_kind: &'static str) {
        let id = Node::compute_id(
            NodeKind::Item,
            &self.repo,
            &self.crate_name,
            Some(&self.file_path),
            name,
            None,
        );
        self.nodes.push(Node {
            id: id.clone(),
            kind: NodeKind::Item,
            name: name.to_string(),
            file_path: Some(self.file_path.clone()),
            crate_name: self.crate_name.clone(),
            repo: self.repo.clone(),
            item_kind: Some(item_kind),
            span: None,
        });
        self.edges.push(Edge {
            src: self.module_id.clone(),
            dst: id,
            kind: EdgeKind::Declares,
            weight: 1,
        });
    }

    fn record_use(&mut self, path: &str, is_pub: bool) {
        // Each `use` produces an unresolved path node + a Uses edge.
        let id = Node::compute_id(NodeKind::Item, &self.repo, "<unresolved>", None, path, None);
        self.nodes.push(Node {
            id: id.clone(),
            kind: NodeKind::Item,
            name: path.to_string(),
            file_path: None,
            crate_name: "<unresolved>".to_string(),
            repo: self.repo.clone(),
            item_kind: Some("use_path"),
            span: None,
        });
        self.edges.push(Edge {
            src: self.module_id.clone(),
            dst: id,
            kind: if is_pub {
                EdgeKind::ReExports
            } else {
                EdgeKind::Uses
            },
            weight: 1,
        });
    }
}

impl<'ast> Visit<'ast> for ItemVisitor {
    fn visit_item_struct(&mut self, i: &'ast syn::ItemStruct) {
        self.record_item(&i.ident.to_string(), "struct");
    }

    fn visit_item_enum(&mut self, i: &'ast syn::ItemEnum) {
        self.record_item(&i.ident.to_string(), "enum");
    }

    fn visit_item_fn(&mut self, i: &'ast syn::ItemFn) {
        self.record_item(&i.sig.ident.to_string(), "fn");
    }

    fn visit_item_trait(&mut self, i: &'ast syn::ItemTrait) {
        self.record_item(&i.ident.to_string(), "trait");
    }

    fn visit_item_impl(&mut self, i: &'ast syn::ItemImpl) {
        // Best-effort impl name: "<TypeName>" or "<TraitName for TypeName>".
        let name = match (&i.trait_, &*i.self_ty) {
            (Some((_, path, _)), syn::Type::Path(p)) => {
                format!("{} for {}", path_to_string(path), path_to_string(&p.path))
            }
            (None, syn::Type::Path(p)) => path_to_string(&p.path),
            _ => "<impl>".to_string(),
        };
        self.record_item(&name, "impl");
    }

    fn visit_item_const(&mut self, i: &'ast syn::ItemConst) {
        self.record_item(&i.ident.to_string(), "const");
    }

    fn visit_item_type(&mut self, i: &'ast syn::ItemType) {
        self.record_item(&i.ident.to_string(), "type");
    }

    fn visit_item_macro(&mut self, i: &'ast syn::ItemMacro) {
        if let Some(ident) = &i.ident {
            self.record_item(&ident.to_string(), "macro");
        }
    }

    fn visit_item_use(&mut self, i: &'ast syn::ItemUse) {
        let is_pub = matches!(i.vis, syn::Visibility::Public(_));
        for path in flatten_use(&i.tree, String::new()) {
            self.record_use(&path, is_pub);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_tmp(contents: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::Builder::new().suffix(".rs").tempfile().unwrap();
        write!(f, "{contents}").unwrap();
        f
    }

    #[test]
    fn parses_struct_and_fn() {
        let f = write_tmp("pub struct Foo;\nfn bar() {}\n");
        let (nodes, edges) =
            parse_file("test-repo", "test-crate", "lib", f.path(), "src/lib.rs").unwrap();
        // 1 module + 1 struct + 1 fn = 3 nodes
        assert_eq!(nodes.len(), 3);
        // 2 declares edges from module
        let declares = edges
            .iter()
            .filter(|e| e.kind == EdgeKind::Declares)
            .count();
        assert_eq!(declares, 2);
    }

    #[test]
    fn parses_use_paths() {
        let f =
            write_tmp("use std::collections::HashMap;\npub use crate::a::B;\nuse foo::{x, y};\n");
        let (_nodes, edges) =
            parse_file("test-repo", "test-crate", "lib", f.path(), "src/lib.rs").unwrap();
        let uses: Vec<&Edge> = edges
            .iter()
            .filter(|e| e.kind == EdgeKind::Uses || e.kind == EdgeKind::ReExports)
            .collect();
        assert!(uses.len() >= 4); // HashMap + B + x + y
        assert!(uses.iter().any(|e| e.kind == EdgeKind::ReExports));
    }
}
