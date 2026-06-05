use super::algorithms_render::{render_algorithm_html, render_index_html};
use super::algorithms_schema::{AlgorithmEntry, ReleaseGateRegistry};
use super::OutputMode;
use anyhow::{anyhow, Context, Result};
use clap::Subcommand;
use convergio_i18n::Bundle;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Algorithm register commands.
#[derive(Subcommand)]
pub(crate) enum AlgorithmsCommand {
    /// Generate a static site tree from a release-gate registry JSON.
    ///
    /// Output layout:
    /// - `<out>/<tenant>/algorithms/index.html`
    /// - `<out>/<tenant>/algorithms/<slug>/index.html`
    Generate {
        /// Path to the release-gate registry JSON document.
        #[arg(long)]
        registry: PathBuf,
        /// Output directory root.
        #[arg(long)]
        out: PathBuf,
        /// Tenant slug (directory name).
        #[arg(long)]
        tenant: String,
    },
}

pub(crate) fn run(bundle: &Bundle, output: OutputMode, cmd: AlgorithmsCommand) -> Result<()> {
    match cmd {
        AlgorithmsCommand::Generate {
            registry,
            out,
            tenant,
        } => generate_algorithms(bundle, output, &registry, &out, &tenant),
    }
}

fn generate_algorithms(
    bundle: &Bundle,
    output: OutputMode,
    registry_path: &Path,
    out_dir: &Path,
    tenant: &str,
) -> Result<()> {
    validate_tenant_slug(tenant)?;

    let raw = fs::read_to_string(registry_path)
        .with_context(|| format!("read registry {}", registry_path.display()))?;
    let doc: ReleaseGateRegistry =
        serde_json::from_str(&raw).with_context(|| "parse registry JSON".to_string())?;

    if doc.schema_version != "1" {
        return Err(anyhow!(
            "unsupported registry schema_version '{}' (expected '1')",
            doc.schema_version
        ));
    }

    if let Some(t) = doc.tenant.as_deref() {
        if t != tenant {
            return Err(anyhow!(
                "registry tenant '{}' does not match --tenant '{}'",
                t,
                tenant
            ));
        }
    }

    validate_algorithms(&doc.algorithms)?;

    let root = out_dir.join(tenant).join("algorithms");
    fs::create_dir_all(&root).with_context(|| format!("create {}", root.display()))?;

    let mut written: Vec<PathBuf> = Vec::new();

    let index_html = render_index_html(tenant, doc.generated_at.as_deref(), &doc.algorithms);
    let index_path = root.join("index.html");
    write_file(&index_path, &index_html)?;
    written.push(index_path);

    for algo in &doc.algorithms {
        let dir = root.join(&algo.slug);
        fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
        let html = render_algorithm_html(tenant, doc.generated_at.as_deref(), algo);
        let path = dir.join("index.html");
        write_file(&path, &html)?;
        written.push(path);
    }

    let report = GenerateReport {
        tenant: tenant.to_string(),
        registry: registry_path.to_path_buf(),
        out_dir: out_dir.to_path_buf(),
        algorithms: doc.algorithms.len(),
        files_written: written,
    };

    render_generate_report(bundle, output, &report);
    Ok(())
}

fn validate_algorithms(algos: &[AlgorithmEntry]) -> Result<()> {
    let mut slugs: HashSet<&str> = HashSet::new();
    let mut actions: HashSet<&str> = HashSet::new();

    for a in algos {
        validate_slug(&a.slug).with_context(|| format!("invalid algorithm slug '{}'", a.slug))?;

        if !slugs.insert(a.slug.as_str()) {
            return Err(anyhow!("duplicate algorithm slug '{}'", a.slug));
        }
        if !actions.insert(a.action.as_str()) {
            return Err(anyhow!("duplicate AI Action '{}'", a.action));
        }
    }

    Ok(())
}

#[derive(Debug, serde::Serialize)]
struct GenerateReport {
    tenant: String,
    registry: PathBuf,
    out_dir: PathBuf,
    algorithms: usize,
    files_written: Vec<PathBuf>,
}

fn render_generate_report(bundle: &Bundle, output: OutputMode, report: &GenerateReport) {
    match output {
        OutputMode::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(report).expect("report JSON")
            );
        }
        OutputMode::Plain => {
            // Stable output for shell scripting.
            let root = report.out_dir.join(&report.tenant).join("algorithms");
            println!("{}", root.display());
        }
        OutputMode::Human => {
            let root = report.out_dir.join(&report.tenant).join("algorithms");
            println!(
                "{}",
                bundle.t(
                    "public-algorithms-generated",
                    &[
                        ("tenant", &report.tenant),
                        ("count", &report.algorithms.to_string()),
                        ("path", &root.to_string_lossy()),
                    ],
                )
            );
        }
    }
}

fn write_file(path: &Path, contents: &str) -> Result<()> {
    fs::write(path, contents).with_context(|| format!("write {}", path.display()))
}

fn validate_tenant_slug(tenant: &str) -> Result<()> {
    validate_slug(tenant).context("invalid tenant slug")
}

fn validate_slug(slug: &str) -> Result<()> {
    if slug.is_empty() {
        return Err(anyhow!("empty slug"));
    }
    if slug == "." || slug == ".." || slug.contains('/') || slug.contains('\\') {
        return Err(anyhow!("unsafe slug"));
    }
    if slug
        .chars()
        .any(|c| !(c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_' | '.')))
    {
        return Err(anyhow!("slug must be lowercase ASCII [a-z0-9._-]"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "algorithms_tests.rs"]
mod algorithms_tests;
