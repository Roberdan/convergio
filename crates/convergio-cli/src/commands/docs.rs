//! `cvg docs ...` — auto-regenerate derived sections of markdown
//! files (ADR-0015).
//!
//! Markdown declares derived blocks via HTML comment markers:
//!
//! ```markdown
//! <!-- BEGIN AUTO:workspace_members -->
//! ...content rewritten by `cvg docs regenerate`...
//! <!-- END AUTO -->
//! ```
//!
//! `regenerate` walks `*.md` under the repo root, looks up each
//! `<name>` in the [`Registry`] of generators, and replaces the
//! block contents. `--check` runs without writing and exits non-zero
//! if anything would change — the CI mirror of
//! `./scripts/generate-docs-index.sh --check`.
//!
//! Rewriter logic (fence handling, line-anchoring) lives in the
//! sibling [`super::docs_rewrite`] module to honour the 300-line cap.

use super::docs_generators::{
    gen_adr_index, gen_cvg_subcommands, gen_test_count, gen_workspace_members,
};
use super::docs_generators_crate::gen_crate_stats;
use super::docs_merge_driver;
use super::docs_rewrite::{rewrite, GeneratorLookup};
use super::OutputMode;
use anyhow::{anyhow, Context, Result};
use clap::Subcommand;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Docs subcommands.
#[derive(Subcommand)]
pub enum DocsCommand {
    /// Rewrite (or check) every `<!-- BEGIN AUTO:... -->` block in
    /// the markdown files under `--root` (default: cwd).
    Regenerate {
        /// Repo root.
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Print what would change but do not write. Exits non-zero
        /// if any block is stale (CI mode).
        #[arg(long)]
        check: bool,
    },
    /// Git merge driver for `*.md` files (P2-9). Git calls:
    /// `cvg docs merge-driver %O %A %B --conflict-marker-size %L --path %P`
    MergeDriver {
        base: PathBuf,
        ours: PathBuf,
        theirs: PathBuf,
        #[arg(long, default_value_t = 7)]
        conflict_marker_size: u32,
        #[arg(long)]
        path: Option<PathBuf>,
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Register the merge driver in `.git/config` and create/update
    /// `.gitattributes` so every `*.md` file uses it on merge.
    InstallMergeDriver {
        /// Repo root (default: cwd).
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
}

/// Entry point.
pub async fn run(output: OutputMode, cmd: DocsCommand) -> Result<()> {
    match cmd {
        DocsCommand::Regenerate { root, check } => regenerate(output, &root, check).await,
        DocsCommand::MergeDriver {
            base,
            ours,
            theirs,
            conflict_marker_size,
            path,
            root,
        } => {
            let code = docs_merge_driver::run_merge_driver(
                &base,
                &ours,
                &theirs,
                conflict_marker_size,
                path.as_deref(),
                &root,
                &Registry::default(),
            )?;
            std::process::exit(code);
        }
        DocsCommand::InstallMergeDriver { root } => {
            docs_merge_driver::install_merge_driver(&root)?;
            if matches!(output, OutputMode::Human) {
                println!("Merge driver installed. Use `git merge` as usual — AUTO blocks resolve automatically.");
            }
            Ok(())
        }
    }
}

async fn regenerate(output: OutputMode, root: &Path, check: bool) -> Result<()> {
    let registry = Registry::default();
    let mut report = Report::default();
    for entry in walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        if path.components().any(|c| {
            matches!(
                c.as_os_str().to_str(),
                Some("target")
                    | Some(".git")
                    | Some("node_modules")
                    | Some(".claude")
                    | Some(".worktrees")
                    | Some("worktrees")
            )
        }) {
            continue;
        }
        let original =
            std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        let rewritten = rewrite(&original, &registry, path, root)?;
        if rewritten != original {
            let rel = path
                .strip_prefix(root)
                .unwrap_or(path)
                .to_string_lossy()
                .into_owned();
            if check {
                report.stale.push(rel);
            } else {
                std::fs::write(path, rewritten)
                    .with_context(|| format!("write {}", path.display()))?;
                report.rewritten.push(rel);
            }
        }
    }

    render(output, &report, check)?;
    if check && !report.stale.is_empty() {
        std::process::exit(1);
    }
    Ok(())
}

#[derive(Debug, Default, serde::Serialize)]
struct Report {
    stale: Vec<String>,
    rewritten: Vec<String>,
}

fn render(output: OutputMode, report: &Report, check: bool) -> Result<()> {
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&report)?),
        OutputMode::Plain => println!(
            "stale={} rewritten={}",
            report.stale.len(),
            report.rewritten.len()
        ),
        OutputMode::Human => {
            if check && !report.stale.is_empty() {
                println!(
                    "{} stale doc file(s) — run `cvg docs regenerate` and commit:",
                    report.stale.len()
                );
                for f in &report.stale {
                    println!("  - {f}");
                }
            } else if !report.rewritten.is_empty() {
                println!("Rewrote {} doc file(s):", report.rewritten.len());
                for f in &report.rewritten {
                    println!("  - {f}");
                }
            } else {
                println!("All AUTO blocks are current.");
            }
        }
    }
    Ok(())
}

/// Generator signature: takes the markdown file currently being
/// rewritten plus the workspace root, returns the fresh body.
type GenFn = fn(&Path, &Path) -> Result<String>;

/// Catalogue of generators. Add a row + a `gen_*` fn in
/// [`super::docs_generators`] when you want a new
/// `<!-- BEGIN AUTO:<name> -->` value to be supported.
struct Registry {
    by_name: BTreeMap<&'static str, GenFn>,
}

impl Default for Registry {
    fn default() -> Self {
        let mut by_name: BTreeMap<&'static str, GenFn> = BTreeMap::new();
        by_name.insert("workspace_members", |_f, r| gen_workspace_members(r));
        by_name.insert("test_count", |_f, r| gen_test_count(r));
        by_name.insert("cvg_subcommands", |_f, r| gen_cvg_subcommands(r));
        by_name.insert("adr_index", |_f, r| gen_adr_index(r));
        by_name.insert("crate_stats", gen_crate_stats);
        Self { by_name }
    }
}

impl GeneratorLookup for Registry {
    fn run(&self, name: &str, file_path: &Path, root: &Path) -> Result<String> {
        let f = self
            .by_name
            .get(name)
            .ok_or_else(|| anyhow!("unknown AUTO generator '{name}'"))?;
        f(file_path, root)
    }
}
