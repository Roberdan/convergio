//! Top-level build pass: ties [`meta`](crate::meta) + [`parse`](crate::parse) + [`store`](crate::store).

use crate::error::Result;
use crate::meta::{snapshot, CrateInfo};
use crate::model::{BuildReport, Edge, EdgeKind, Node};
use crate::parse::parse_file;
use crate::store::{is_stale, Store};
use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Run a full or incremental build pass against a workspace.
///
/// `manifest_dir` is the root containing the workspace `Cargo.toml`.
/// The repo name is derived from the manifest directory's basename.
/// `force` skips the mtime check and re-parses every file.
pub async fn build(manifest_dir: &Path, store: &Store, force: bool) -> Result<BuildReport> {
    store.migrate().await?;

    let repo = repo_name(manifest_dir);
    let snap = snapshot(manifest_dir, &repo)?;
    for n in &snap.nodes {
        store.upsert_node(n).await?;
    }
    for e in &snap.edges {
        store.upsert_edge(e).await?;
    }

    let stored_mtimes = store.file_mtimes().await?;
    let mut report = BuildReport {
        nodes: 0,
        edges: 0,
        crates: snap.crates.len(),
        files_parsed: 0,
        files_skipped: 0,
        doc_edges_skipped: 0,
    };

    for c in &snap.crates {
        for entry in WalkDir::new(&c.src_root)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !is_rust_file(path) {
                continue;
            }
            let rel = relativise(manifest_dir, path);
            let stored = stored_mtimes.get(&rel);
            if !force && !is_stale(path, stored) {
                report.files_skipped += 1;
                continue;
            }
            let module_path = module_path_from_file(&c.src_root, path);
            let (nodes, edges) = parse_file(&repo, &c.name, &module_path, path, &rel)?;
            let mtime = current_mtime(path)?;
            // Bridge module → crate so `cvg graph for-task` can walk
            // from a file back to its crate node.
            let crate_id_node = c_node_for_id(&snap.crates, &c.name, &repo);
            let edges_with_bridge = bridge_module_to_crate(&nodes, crate_id_node, edges);
            store
                .upsert_file(&rel, mtime, &nodes, &edges_with_bridge)
                .await?;
            report.files_parsed += 1;
        }
    }

    // Scan docs/ for ADRs and other markdown so frontmatter
    // claims/mentions show up as graph edges. Failure to walk docs
    // is non-fatal — the code-side graph is still valuable.
    let docs_dir = manifest_dir.join("docs");
    if docs_dir.exists() {
        scan_docs(manifest_dir, &docs_dir, &repo, store, &mut report).await?;
    }

    report.nodes = store.count_nodes().await?;
    report.edges = store.count_edges().await?;
    Ok(report)
}

async fn scan_docs(
    manifest_dir: &Path,
    docs_dir: &Path,
    repo: &str,
    store: &Store,
    report: &mut BuildReport,
) -> Result<()> {
    use crate::doc_link::parse_doc;
    // Two passes so a mentions-edge to ADR `B` resolves even when
    // ADR `B` is parsed *after* the ADR that mentions it.
    // Pass 1: collect all (rel_path, mtime, node, edges) and upsert
    // every node first. Pass 2: upsert the edges.
    struct DocBundle {
        rel: String,
        mtime: DateTime<Utc>,
        node: Node,
        edges: Vec<Edge>,
    }
    let mut bundles: Vec<DocBundle> = Vec::new();
    for entry in WalkDir::new(docs_dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.extension().is_some_and(|e| e == "md") {
            continue;
        }
        let rel = relativise(manifest_dir, path);
        let mtime = current_mtime(path)?;
        let (node, edges) = parse_doc(repo, &rel, path)?;
        bundles.push(DocBundle {
            rel,
            mtime,
            node,
            edges,
        });
    }
    // Pass 1: drop+insert each doc's node (no edges yet).
    for b in &bundles {
        store
            .upsert_file(&b.rel, b.mtime, std::slice::from_ref(&b.node), &[])
            .await?;
    }
    // Pass 2: insert edges now that every src/dst node exists. Skip
    // edges whose dst is unknown (e.g. ADR mentions a crate that no
    // longer ships) — better to drop than to refuse the build, but
    // count and warn so broken doc-link invariants surface instead of
    // disappearing at debug level (audit 2026-05 follow-up).
    for b in &bundles {
        for e in &b.edges {
            if let Err(err) = store.upsert_edge(e).await {
                report.doc_edges_skipped += 1;
                tracing::warn!(
                    src = %e.src,
                    dst = %e.dst,
                    kind = e.kind.as_str(),
                    doc = %b.rel,
                    error = %err,
                    "skipping doc edge with unknown destination"
                );
            }
        }
        report.files_parsed += 1;
    }
    Ok(())
}

fn is_rust_file(p: &Path) -> bool {
    p.extension().is_some_and(|e| e == "rs")
}

fn current_mtime(p: &Path) -> Result<DateTime<Utc>> {
    let m = std::fs::metadata(p)?.modified()?;
    Ok(m.into())
}

fn relativise(root: &Path, p: &Path) -> String {
    p.strip_prefix(root)
        .unwrap_or(p)
        .to_string_lossy()
        .into_owned()
}

fn module_path_from_file(src_root: &Path, file: &Path) -> String {
    let rel = file.strip_prefix(src_root).unwrap_or(file);
    let mut parts: Vec<String> = Vec::new();
    let total = rel.components().count();
    for (i, comp) in rel.components().enumerate() {
        let s = comp.as_os_str().to_string_lossy().to_string();
        if i + 1 == total {
            // Last component: strip ".rs" extension; "lib.rs" / "main.rs"
            // become root markers.
            let stem = PathBuf::from(&s)
                .file_stem()
                .map(|os| os.to_string_lossy().into_owned())
                .unwrap_or(s.clone());
            if stem == "lib" || stem == "main" || stem == "mod" {
                continue;
            }
            parts.push(stem);
        } else {
            parts.push(s);
        }
    }
    parts.join("::")
}

fn c_node_for_id(crates: &[CrateInfo], name: &str, repo: &str) -> String {
    use crate::model::{Node, NodeKind};
    let _ = crates;
    Node::compute_id(NodeKind::Crate, repo, name, None, name, None)
}

fn repo_name(manifest_dir: &Path) -> String {
    manifest_dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unknown".to_string())
}

fn bridge_module_to_crate(nodes: &[Node], crate_id: String, mut edges: Vec<Edge>) -> Vec<Edge> {
    if let Some(module) = nodes
        .iter()
        .find(|n| matches!(n.kind, crate::model::NodeKind::Module))
    {
        edges.push(Edge {
            src: crate_id,
            dst: module.id.clone(),
            kind: EdgeKind::Declares,
            weight: 1,
        });
    }
    edges
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_path_for_lib_root_is_empty() {
        let src = Path::new("/repo/crates/x/src");
        let f = Path::new("/repo/crates/x/src/lib.rs");
        assert_eq!(module_path_from_file(src, f), "");
    }

    #[test]
    fn module_path_for_nested_module() {
        let src = Path::new("/repo/crates/x/src");
        let f = Path::new("/repo/crates/x/src/commands/session.rs");
        assert_eq!(module_path_from_file(src, f), "commands::session");
    }

    #[test]
    fn module_path_for_mod_rs_uses_dir() {
        let src = Path::new("/repo/crates/x/src");
        let f = Path::new("/repo/crates/x/src/commands/mod.rs");
        assert_eq!(module_path_from_file(src, f), "commands");
    }
}
