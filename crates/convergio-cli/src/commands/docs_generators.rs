//! AUTO-block generators for `cvg docs regenerate` (ADR-0015).
//!
//! Each `gen_*` fn produces the body of a single
//! `<!-- BEGIN AUTO:<name> --> ... <!-- END AUTO -->` block. They
//! live here so the registry in [`super::docs`] stays a thin index
//! and this file can grow as more derived sections move from the
//! "we keep forgetting to update this" pile to the "the daemon
//! regenerates it on every commit" pile.

use anyhow::{Context, Result};
use cargo_metadata::MetadataCommand;
use std::path::Path;

/// Workspace member list — `<crate-name> — <one-line description>`.
pub(super) fn gen_workspace_members(root: &Path) -> Result<String> {
    let manifest = root.join("Cargo.toml");
    let meta = MetadataCommand::new()
        .manifest_path(&manifest)
        .no_deps()
        .exec()
        .context("cargo metadata --no-deps")?;
    let mut crates: Vec<&cargo_metadata::Package> = meta
        .workspace_members
        .iter()
        .filter_map(|id| meta.packages.iter().find(|p| &p.id == id))
        .collect();
    crates.sort_by(|a, b| a.name.cmp(&b.name));
    let mut out = String::new();
    for pkg in crates {
        let desc = pkg
            .description
            .as_deref()
            .map(|s| s.split('.').next().unwrap_or(s).trim().to_string())
            .unwrap_or_else(|| String::from("(no description)"));
        out.push_str(&format!("- `{}` — {}\n", pkg.name, desc));
    }
    Ok(out)
}

/// Number of declared `#[test]` and `#[tokio::test]` functions across
/// the workspace. Source-of-truth disclaimer baked into the output so
/// readers know `cargo test --workspace` remains authoritative.
pub(super) fn gen_test_count(root: &Path) -> Result<String> {
    let crates_dir = root.join("crates");
    let mut count = 0_usize;
    if crates_dir.is_dir() {
        for entry in walkdir::WalkDir::new(&crates_dir)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_file() || path.extension().and_then(|s| s.to_str()) != Some("rs") {
                continue;
            }
            if path
                .components()
                .any(|c| matches!(c.as_os_str().to_str(), Some("target")))
            {
                continue;
            }
            let Ok(src) = std::fs::read_to_string(path) else {
                continue;
            };
            for line in src.lines() {
                let t = line.trim_start();
                if t == "#[test]" || t == "#[tokio::test]" || t.starts_with("#[tokio::test(") {
                    count += 1;
                }
            }
        }
    }
    Ok(format!(
        "**Tests declared:** {count} (counted from `#[test]` + `#[tokio::test]` annotations \
         under `crates/`; live runner count via `cargo test --workspace`).\n"
    ))
}

/// Top-level `cvg` subcommands, derived from the public modules in
/// `crates/convergio-cli/src/commands/mod.rs`. The list is kept stable
/// across runs by sorting alphabetically.
pub(super) fn gen_cvg_subcommands(root: &Path) -> Result<String> {
    let mod_rs = root.join("crates/convergio-cli/src/commands/mod.rs");
    let src =
        std::fs::read_to_string(&mod_rs).with_context(|| format!("read {}", mod_rs.display()))?;
    let mut names: Vec<&str> = src
        .lines()
        .filter_map(|l| {
            let t = l.trim();
            let rest = t.strip_prefix("pub mod ")?;
            let name = rest.split(';').next()?.trim();
            if name.is_empty() {
                None
            } else {
                Some(name)
            }
        })
        .collect();
    names.sort();
    names.dedup();
    let mut out = String::new();
    for n in names {
        out.push_str(&format!("- `cvg {}`\n", n.replace('_', "-")));
    }
    Ok(out)
}

/// MADR index for `docs/adr/`. Outputs a markdown table with the same
/// shape the index already used (number / title / status) so the
/// AUTO block can wrap the existing prose without a layout change.
pub(super) fn gen_adr_index(root: &Path) -> Result<String> {
    let adr_dir = root.join("docs/adr");
    let mut rows: Vec<(String, String, String)> = Vec::new();
    if adr_dir.is_dir() {
        for entry in std::fs::read_dir(&adr_dir)
            .with_context(|| format!("read_dir {}", adr_dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            if !name.ends_with(".md") || name == "README.md" {
                continue;
            }
            let id = match name.split('-').next() {
                Some(s) if s.chars().all(|c| c.is_ascii_digit()) && s.len() == 4 => s.to_string(),
                _ => continue,
            };
            if id == "0000" {
                continue; // template
            }
            let src = std::fs::read_to_string(&path)
                .with_context(|| format!("read {}", path.display()))?;
            let status = parse_frontmatter_field(&src, "status").unwrap_or_else(|| "?".into());
            let title = first_h1(&src).unwrap_or_else(|| name.to_string());
            rows.push((id, title, status));
        }
    }
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    let mut out = String::new();
    out.push_str("| # | Title | Status |\n");
    out.push_str("|---|-------|--------|\n");
    for (id, title, status) in rows {
        // Strip a leading "ADR-NNNN: " prefix from the H1 if present so
        // the table column carries only the human title.
        let display = title
            .trim_start_matches(&format!("ADR-{id}:"))
            .trim()
            .trim_start_matches("ADR")
            .trim_start_matches(':')
            .trim()
            .to_string();
        let display = if display.is_empty() { title } else { display };
        out.push_str(&format!(
            "| [{id}](./{id_full}.md) | {display} | {status} |\n",
            id_full = adr_filename(&adr_dir, &id).unwrap_or_else(|| format!("{id}-")),
        ));
    }
    Ok(out)
}

fn parse_frontmatter_field(src: &str, field: &str) -> Option<String> {
    let mut lines = src.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }
    for line in lines {
        let t = line.trim();
        if t == "---" {
            break;
        }
        if let Some(rest) = t.strip_prefix(&format!("{field}:")) {
            return Some(rest.trim().to_string());
        }
    }
    None
}

fn first_h1(src: &str) -> Option<String> {
    for line in src.lines() {
        if let Some(rest) = line.strip_prefix("# ") {
            return Some(rest.trim().to_string());
        }
    }
    None
}

fn adr_filename(adr_dir: &Path, id: &str) -> Option<String> {
    let entries = std::fs::read_dir(adr_dir).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_str()?;
        if name.starts_with(&format!("{id}-")) && name.ends_with(".md") {
            return Some(name.trim_end_matches(".md").to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_frontmatter_picks_field() {
        let src = "---\nid: 0014\nstatus: accepted\n---\n# title\n";
        assert_eq!(
            parse_frontmatter_field(src, "status").as_deref(),
            Some("accepted")
        );
        assert_eq!(parse_frontmatter_field(src, "id").as_deref(), Some("0014"));
        assert_eq!(parse_frontmatter_field(src, "missing"), None);
    }

    #[test]
    fn first_h1_returns_inline_title() {
        let src = "---\nid: 1\n---\n# Hello world\nmore\n";
        assert_eq!(first_h1(src).as_deref(), Some("Hello world"));
    }
}
