//! `cargo_metadata` wrapper: produces crate-level nodes and
//! depends_on edges between workspace members.

use crate::error::Result;
use crate::model::{Edge, EdgeKind, Node, NodeKind};
use cargo_metadata::MetadataCommand;
use std::collections::BTreeMap;
use std::path::Path;

/// One crate the parser will visit, with its workspace path and the
/// list of `*.rs` files we want to walk.
pub struct CrateInfo {
    /// Crate name as declared in Cargo.toml.
    pub name: String,
    /// Filesystem path to the crate root (the dir containing `Cargo.toml`).
    pub root: std::path::PathBuf,
    /// Source root (`<root>/src`).
    pub src_root: std::path::PathBuf,
}

/// Result of a metadata pass.
pub struct MetaSnapshot {
    /// Workspace member crates.
    pub crates: Vec<CrateInfo>,
    /// Crate-level nodes (one per workspace member).
    pub nodes: Vec<Node>,
    /// `depends_on` edges between workspace members.
    pub edges: Vec<Edge>,
}

/// Run `cargo metadata` against `manifest_dir/Cargo.toml` and return
/// the workspace member info plus crate-level graph nodes/edges.
pub fn snapshot(manifest_dir: &Path, repo: &str) -> Result<MetaSnapshot> {
    let manifest = manifest_dir.join("Cargo.toml");
    let meta = MetadataCommand::new().manifest_path(&manifest).exec()?;

    // Map crate name -> stable node id, so the dependency edges
    // reference the same id as the crate node.
    let mut id_for: BTreeMap<String, String> = BTreeMap::new();
    let mut crates: Vec<CrateInfo> = Vec::new();
    let mut nodes: Vec<Node> = Vec::new();

    for member_id in &meta.workspace_members {
        let pkg = meta
            .packages
            .iter()
            .find(|p| &p.id == member_id)
            .ok_or_else(|| {
                crate::error::GraphError::Other(format!("workspace member {member_id} not found"))
            })?;
        let root = pkg
            .manifest_path
            .parent()
            .ok_or_else(|| crate::error::GraphError::Other("manifest has no parent".into()))?
            .to_path_buf()
            .into_std_path_buf();
        let src_root = root.join("src");
        let info = CrateInfo {
            name: pkg.name.clone(),
            root,
            src_root,
        };
        let id = Node::compute_id(NodeKind::Crate, repo, &info.name, None, &info.name, None);
        id_for.insert(info.name.clone(), id.clone());
        nodes.push(Node {
            id,
            kind: NodeKind::Crate,
            name: info.name.clone(),
            file_path: None,
            crate_name: info.name.clone(),
            repo: repo.to_string(),
            item_kind: None,
            span: None,
        });
        crates.push(info);
    }

    let mut edges: Vec<Edge> = Vec::new();
    for member_id in &meta.workspace_members {
        let Some(pkg) = meta.packages.iter().find(|p| &p.id == member_id) else {
            return Err(crate::error::GraphError::Other(format!(
                "workspace member {member_id} not found"
            )));
        };
        let Some(src_id) = id_for.get(&pkg.name) else {
            continue;
        };
        for dep in &pkg.dependencies {
            if let Some(dst_id) = id_for.get(&dep.name) {
                edges.push(Edge {
                    src: src_id.clone(),
                    dst: dst_id.clone(),
                    kind: EdgeKind::DependsOn,
                    weight: 1,
                });
            }
        }
    }

    Ok(MetaSnapshot {
        crates,
        nodes,
        edges,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_finds_workspace_members() {
        let here = Path::new(env!("CARGO_MANIFEST_DIR"));
        let workspace = here.parent().unwrap().parent().unwrap();
        let snap = snapshot(workspace, "convergio").unwrap();
        assert!(snap.crates.iter().any(|c| c.name == "convergio-graph"));
        assert!(snap.crates.iter().any(|c| c.name == "convergio-cli"));
        assert!(!snap.nodes.is_empty());
        // edges may be empty if a crate has no internal deps; just sanity-check shape
        for e in &snap.edges {
            assert_eq!(e.kind, EdgeKind::DependsOn);
        }
    }
}
