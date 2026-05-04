//! ADR / index / workspace parsers for [`crate::coherence`].
//!
//! Split out to honour the 300-line per-file cap.

use anyhow::{Context, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

#[derive(Debug)]
pub(super) struct Adr {
    pub(super) id: String,
    pub(super) path: String,
    pub(super) status: String,
    pub(super) related_adrs: Vec<String>,
    pub(super) touches_crates: Vec<String>,
}

#[derive(Default)]
pub(super) struct Frontmatter {
    pub(super) status: String,
    pub(super) related_adrs: Vec<String>,
    pub(super) touches_crates: Vec<String>,
}

pub(super) fn load_adrs(dir: &Path) -> Result<Vec<Adr>> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir).with_context(|| format!("read_dir {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) if n.ends_with(".md") && n != "README.md" => n.to_string(),
            _ => continue,
        };
        let id = name.split('-').next().unwrap_or("").to_string();
        if !id.chars().all(|c| c.is_ascii_digit()) || id.is_empty() {
            continue;
        }
        // 0000 is the template, not a real ADR. It is intentionally
        // absent from docs/adr/README.md, so skip the coherence check.
        if id == "0000" {
            continue;
        }
        let body = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let fm = parse_frontmatter(&body).with_context(|| format!("frontmatter {name}"))?;
        out.push(Adr {
            id,
            path: path
                .strip_prefix(dir.parent().unwrap_or(dir))
                .unwrap_or(&path)
                .to_string_lossy()
                .into_owned(),
            status: fm.status,
            related_adrs: fm.related_adrs,
            touches_crates: fm.touches_crates,
        });
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

pub(super) fn parse_frontmatter(body: &str) -> Result<Frontmatter> {
    let mut lines = body.lines().peekable();
    if lines.next().map(str::trim) != Some("---") {
        anyhow::bail!("missing opening --- delimiter");
    }
    let mut fm = Frontmatter::default();
    while let Some(line) = lines.next() {
        let trimmed = line.trim_end();
        if trimmed.trim() == "---" {
            return Ok(fm);
        }
        let Some((key, value)) = trimmed.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "status" => fm.status = value.trim_matches('"').to_string(),
            "related_adrs" => fm.related_adrs = read_yaml_list(value, &mut lines),
            "touches_crates" => fm.touches_crates = read_yaml_list(value, &mut lines),
            _ => {}
        }
    }
    anyhow::bail!("missing closing --- delimiter")
}

/// Read a YAML list, supporting both inline `[a, b]` and block:
///
/// ```yaml
/// key:
///   - a
///   - b
/// ```
///
/// `value` is what already followed the `:` on the key line. If
/// non-empty (e.g. `[a, b]` or `[]`), it is parsed inline. Otherwise
/// we consume subsequent block-list rows from `lines` until we hit a
/// non-`-` line, then put it back via the iterator's natural advance.
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

pub(super) fn parse_index(path: &Path) -> Result<BTreeMap<String, String>> {
    let body = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let mut map = BTreeMap::new();
    for line in body.lines() {
        if !line.starts_with("| [") {
            continue;
        }
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        if cells.len() < 4 {
            continue;
        }
        let id_cell = cells[1];
        let status_cell = cells[3];
        let id = id_cell
            .trim_start_matches("[")
            .split(']')
            .next()
            .unwrap_or("")
            .to_string();
        if id.chars().all(|c| c.is_ascii_digit()) && !id.is_empty() {
            map.insert(id, status_cell.to_string());
        }
    }
    Ok(map)
}

/// Public-ish ADR view used by the `cvg coherence adrs` sub-verifier.
///
/// Differs from the internal [`Adr`] in two ways: the slug (filename
/// minus the leading `NNNN-`) is exposed for keyword extraction, and
/// no `path` / `related_adrs` fields are carried — the ADR
/// status-vs-implementation cross-check does not need them.
#[derive(Debug)]
pub struct AdrFull {
    /// Zero-padded ADR id (`"0006"`).
    pub id: String,
    /// Filename slug after the id (`"crdt-storage"` for
    /// `0006-crdt-storage.md`).
    pub slug: String,
    /// Verbatim status string from frontmatter.
    pub status: String,
    /// `touches_crates` from frontmatter (may be empty).
    pub touches_crates: Vec<String>,
}

/// Like [`load_adrs`], but also exposes the filename slug. Used by
/// [`crate::adrs`] for keyword extraction.
pub fn load_adrs_full(dir: &Path) -> Result<Vec<AdrFull>> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir).with_context(|| format!("read_dir {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) if n.ends_with(".md") && n != "README.md" => n.to_string(),
            _ => continue,
        };
        let stem = name.trim_end_matches(".md");
        let (id, slug) = match stem.split_once('-') {
            Some((i, s)) => (i.to_string(), s.to_string()),
            None => continue,
        };
        if !id.chars().all(|c| c.is_ascii_digit()) || id.is_empty() {
            continue;
        }
        if id == "0000" {
            continue;
        }
        let body = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let fm = parse_frontmatter(&body).with_context(|| format!("frontmatter {name}"))?;
        out.push(AdrFull {
            id,
            slug,
            status: fm.status,
            touches_crates: fm.touches_crates,
        });
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

pub(super) fn parse_workspace_members(path: &Path) -> Result<BTreeSet<String>> {
    let body = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let parsed: toml::Value = body.parse().context("parse Cargo.toml")?;
    let members = parsed
        .get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(toml::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("workspace.members not found"))?;
    let mut out = BTreeSet::new();
    for m in members {
        if let Some(s) = m.as_str() {
            // Member entries are paths like "crates/convergio-cli".
            let name = s.rsplit('/').next().unwrap_or(s).to_string();
            out.insert(name);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_frontmatter_extracts_lists() {
        let body = "---\nid: 0001\nstatus: accepted\nrelated_adrs: [0002, 0003]\ntouches_crates: [convergio-cli]\n---\n# body\n";
        let fm = parse_frontmatter(body).unwrap();
        assert_eq!(fm.status, "accepted");
        assert_eq!(fm.related_adrs, vec!["0002", "0003"]);
        assert_eq!(fm.touches_crates, vec!["convergio-cli"]);
    }

    #[test]
    fn parse_frontmatter_handles_empty_lists() {
        let body = "---\nstatus: proposed\nrelated_adrs: []\ntouches_crates: []\n---\n";
        let fm = parse_frontmatter(body).unwrap();
        assert!(fm.related_adrs.is_empty());
        assert!(fm.touches_crates.is_empty());
    }

    #[test]
    fn parse_frontmatter_handles_block_lists() {
        let body = "---\nstatus: accepted\nrelated_adrs:\n  - 0001\n  - 0002\ntouches_crates:\n  - convergio-cli\n  - convergio-i18n\n---\n";
        let fm = parse_frontmatter(body).unwrap();
        assert_eq!(fm.related_adrs, vec!["0001", "0002"]);
        assert_eq!(fm.touches_crates, vec!["convergio-cli", "convergio-i18n"]);
    }

    #[test]
    fn parse_frontmatter_handles_mixed_list_styles() {
        let body = "---\nrelated_adrs: [0001]\ntouches_crates:\n  - convergio-cli\n---\n";
        let fm = parse_frontmatter(body).unwrap();
        assert_eq!(fm.related_adrs, vec!["0001"]);
        assert_eq!(fm.touches_crates, vec!["convergio-cli"]);
    }
}
