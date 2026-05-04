//! ADR / markdown frontmatter parsing — produces graph nodes (kind
//! = `Adr` / `Doc`) and `claims` edges to the crate nodes referenced
//! in `touches_crates`.
//!
//! v0 scope: only YAML frontmatter is parsed; no full-text scan for
//! symbol mentions yet (PR 14.3 territory).

use crate::error::Result;
use crate::model::{Edge, EdgeKind, Node, NodeKind, DOCS_CRATE};
use std::collections::BTreeSet;
use std::path::Path;

/// Parse one ADR or markdown file. Returns the doc node + edges
/// (claims to crates listed in `touches_crates`, mentions to other
/// ADRs listed in `related_adrs`).
pub fn parse_doc(repo: &str, rel_path: &str, abs_path: &Path) -> Result<(Node, Vec<Edge>)> {
    let body = std::fs::read_to_string(abs_path)?;
    let fm = parse_frontmatter(&body);

    let kind = if rel_path.contains("/adr/") {
        NodeKind::Adr
    } else {
        NodeKind::Doc
    };
    let name = fm
        .id
        .clone()
        .unwrap_or_else(|| filename_without_ext(rel_path));
    let id = Node::compute_id(kind, repo, DOCS_CRATE, Some(rel_path), &name, None);
    let node = Node {
        id: id.clone(),
        kind,
        name,
        file_path: Some(rel_path.to_string()),
        crate_name: DOCS_CRATE.to_string(),
        repo: repo.to_string(),
        item_kind: None,
        span: None,
    };

    let mut edges: Vec<Edge> = Vec::new();
    for crate_name in &fm.touches_crates {
        let dst = Node::compute_id(NodeKind::Crate, repo, crate_name, None, crate_name, None);
        edges.push(Edge {
            src: id.clone(),
            dst,
            kind: EdgeKind::Claims,
            weight: 1,
        });
    }
    for other_adr in &fm.related_adrs {
        // ADR ids in frontmatter look like "0001"; the matching node
        // name is also "0001" so the id resolves identically.
        let dst = Node::compute_id(NodeKind::Adr, repo, DOCS_CRATE, None, other_adr, None);
        edges.push(Edge {
            src: id.clone(),
            dst,
            kind: EdgeKind::Mentions,
            weight: 1,
        });
    }
    Ok((node, edges))
}

#[derive(Default)]
pub(crate) struct DocFrontmatter {
    pub(crate) id: Option<String>,
    pub(crate) touches_crates: Vec<String>,
    pub(crate) related_adrs: Vec<String>,
}

/// Extract YAML frontmatter from a markdown body. Tolerates missing
/// frontmatter (returns an empty struct) so non-ADR docs still
/// produce a node, just without claims/mentions edges.
pub(crate) fn parse_frontmatter(body: &str) -> DocFrontmatter {
    let mut out = DocFrontmatter::default();
    let mut lines = body.lines().peekable();
    if lines.next().map(str::trim) != Some("---") {
        return out;
    }
    while let Some(line) = lines.next() {
        let trimmed = line.trim_end();
        if trimmed.trim() == "---" {
            return out;
        }
        let Some((key, value)) = trimmed.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "id" => out.id = Some(value.trim_matches('"').to_string()),
            "related_adrs" => out.related_adrs = read_yaml_list(value, &mut lines),
            "touches_crates" => out.touches_crates = read_yaml_list(value, &mut lines),
            _ => {}
        }
    }
    out
}

fn read_yaml_list<'a, I>(value: &str, lines: &mut std::iter::Peekable<I>) -> Vec<String>
where
    I: Iterator<Item = &'a str>,
{
    if !value.is_empty() {
        return parse_inline_list(value);
    }
    let mut out = Vec::new();
    while let Some(peek) = lines.peek() {
        let trimmed = peek.trim_start();
        if !trimmed.starts_with("- ") && trimmed != "-" {
            break;
        }
        let item = trimmed
            .trim_start_matches('-')
            .trim()
            .trim_matches('"')
            .to_string();
        if !item.is_empty() {
            out.push(item);
        }
        lines.next();
    }
    out
}

fn parse_inline_list(value: &str) -> Vec<String> {
    let inside = value.trim().trim_start_matches('[').trim_end_matches(']');
    if inside.trim().is_empty() {
        return Vec::new();
    }
    inside
        .split(',')
        .map(|s| s.trim().trim_matches('"').to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn filename_without_ext(rel_path: &str) -> String {
    let base = rel_path.rsplit('/').next().unwrap_or(rel_path);
    base.rsplit_once('.')
        .map(|(s, _)| s)
        .unwrap_or(base)
        .to_string()
}

/// Walk a docs root and return the set of relative ADR/doc paths.
pub fn walk_docs(root: &Path) -> Result<BTreeSet<String>> {
    let mut out = BTreeSet::new();
    for entry in walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let p = entry.path();
        if p.extension().is_some_and(|e| e == "md") {
            let rel = p
                .strip_prefix(root.parent().unwrap_or(root))
                .unwrap_or(p)
                .to_string_lossy()
                .into_owned();
            out.insert(rel);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_tmp(contents: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::Builder::new().suffix(".md").tempfile().unwrap();
        write!(f, "{contents}").unwrap();
        f
    }

    #[test]
    fn parses_adr_with_inline_lists() {
        let f = write_tmp(
            "---\nid: 0014\ntouches_crates: [convergio-graph, convergio-cli]\nrelated_adrs: [0001, 0002]\n---\n# body",
        );
        let (node, edges) = parse_doc("convergio", "docs/adr/0014-foo.md", f.path()).unwrap();
        assert_eq!(node.kind, NodeKind::Adr);
        assert_eq!(node.name, "0014");
        let claims: Vec<&Edge> = edges
            .iter()
            .filter(|e| e.kind == EdgeKind::Claims)
            .collect();
        assert_eq!(claims.len(), 2);
        let mentions: Vec<&Edge> = edges
            .iter()
            .filter(|e| e.kind == EdgeKind::Mentions)
            .collect();
        assert_eq!(mentions.len(), 2);
    }

    #[test]
    fn parses_block_lists() {
        let f = write_tmp(
            "---\nid: 0099\ntouches_crates:\n  - foo\n  - bar\nrelated_adrs:\n  - 0001\n---\n",
        );
        let (_node, edges) = parse_doc("convergio", "docs/adr/0099-x.md", f.path()).unwrap();
        let claims = edges.iter().filter(|e| e.kind == EdgeKind::Claims).count();
        assert_eq!(claims, 2);
    }

    #[test]
    fn handles_no_frontmatter() {
        let f = write_tmp("# Just a doc\nNo frontmatter here.\n");
        let (node, edges) = parse_doc("convergio", "docs/foo.md", f.path()).unwrap();
        assert_eq!(node.kind, NodeKind::Doc);
        assert!(edges.is_empty());
    }
}
