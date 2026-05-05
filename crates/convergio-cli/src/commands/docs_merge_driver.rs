//! Git merge driver for `*.md` AUTO-block files (P2-9 / ADR-0015).
//! Git calls: `cvg docs merge-driver %O %A %B --conflict-marker-size %L --path %P`
//! Exit 0 = clean; 1 = unresolved conflicts remain.

use super::docs_rewrite::{rewrite, strip_auto_conflicts, GeneratorLookup};
use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;

/// Run `git merge-file`, strip conflicts inside AUTO blocks, regenerate.
/// Returns 0 (clean) or 1 (conflicts remain outside AUTO blocks).
pub(super) fn run_merge_driver<G: GeneratorLookup>(
    base: &Path,
    ours: &Path,
    theirs: &Path,
    conflict_marker_size: u32,
    path: Option<&Path>,
    root: &Path,
    registry: &G,
) -> Result<i32> {
    Command::new("git")
        .args(["merge-file", "-q"])
        .arg(format!("--marker-size={conflict_marker_size}"))
        .arg(ours)
        .arg(base)
        .arg(theirs)
        .status()
        .context("spawn git merge-file")?;
    let merged =
        std::fs::read_to_string(ours).with_context(|| format!("read {}", ours.display()))?;
    let resolved = strip_auto_conflicts(&merged, conflict_marker_size);
    let final_content = rewrite(&resolved, registry, path.unwrap_or(ours), root)?;
    std::fs::write(ours, &final_content).with_context(|| format!("write {}", ours.display()))?;
    let start = "<".repeat(conflict_marker_size as usize);
    Ok(if final_content.contains(&start) { 1 } else { 0 })
}

/// Register the driver in `.git/config` and create/update `.gitattributes`.
pub(super) fn install_merge_driver(root: &Path) -> Result<()> {
    git_cfg(
        root,
        "merge.cvg-auto-blocks.name",
        "Convergio AUTO-blocks merge driver",
    )?;
    git_cfg(
        root,
        "merge.cvg-auto-blocks.driver",
        "cvg docs merge-driver %O %A %B --conflict-marker-size %L --path %P",
    )?;
    let ga = root.join(".gitattributes");
    let existing = std::fs::read_to_string(&ga).unwrap_or_default();
    if !existing.contains("merge=cvg-auto-blocks") {
        std::fs::write(&ga, format!("{existing}*.md merge=cvg-auto-blocks\n"))
            .context("write .gitattributes")?;
    }
    Ok(())
}

fn git_cfg(root: &Path, key: &str, value: &str) -> Result<()> {
    let s = Command::new("git")
        .args(["config", key, value])
        .current_dir(root)
        .status()
        .with_context(|| format!("git config {key}"))?;
    if !s.success() {
        bail!("git config {key} failed");
    }
    Ok(())
}
