//! `cvg setup fleet` — bootstrap the operator daemon's fleet (P0-2,
//! findings H1+H2 from the 2026-05-04 retrospective).
//!
//! On first run: scan `~/GitHub/convergio*` for git repos, register
//! each via `POST /v1/fleet/repos`, then `POST /v1/fleet/build` with
//! `refresh_similarity=true`. Idempotent: the daemon's add route is
//! already idempotent (409 on duplicate name → treated as success
//! here), so re-running picks up new repos without breaking the
//! existing index.

use super::{Client, OutputMode};
use anyhow::Result;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Run the bootstrap.
pub async fn run(client: &Client, output: OutputMode, force: bool) -> Result<()> {
    let _ = force;
    let candidates = discover_candidates();
    if candidates.is_empty() {
        println!("cvg setup fleet — no ~/GitHub/convergio* repos found.");
        return Ok(());
    }
    println!(
        "cvg setup fleet — scanning {} candidate(s):",
        candidates.len()
    );
    let mut added = 0usize;
    let mut already = 0usize;
    for path in &candidates {
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("repo")
            .to_string();
        match register_repo(client, path, &name).await {
            Ok(true) => {
                println!("  + added {} ({})", name, path.display());
                added += 1;
            }
            Ok(false) => {
                println!("  · already registered: {}", name);
                already += 1;
            }
            Err(e) => println!("  ! {}: {}", name, e),
        }
    }
    if added == 0 && already > 0 {
        println!(
            "\n  fleet bootstrap idempotent re-run: {} repo(s) already present.",
            already
        );
    }
    println!("\n  running fleet build (this may take a few minutes)...");
    match client
        .post::<Value, Value>("/v1/fleet/build", &json!({"refresh_similarity": true}))
        .await
    {
        Ok(stats) => match output {
            OutputMode::Json => println!("{}", serde_json::to_string_pretty(&stats)?),
            _ => render_build_summary(&stats),
        },
        Err(e) => println!("  ! fleet build failed: {e}"),
    }
    Ok(())
}

fn discover_candidates() -> Vec<PathBuf> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let github = Path::new(&home).join("GitHub");
    let Ok(entries) = std::fs::read_dir(&github) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if !name.starts_with("convergio") {
            continue;
        }
        if !path.join(".git").exists() {
            continue;
        }
        out.push(path);
    }
    out.sort();
    out
}

async fn register_repo(client: &Client, path: &Path, name: &str) -> Result<bool> {
    let language = detect_language(path);
    let parser = default_parser(&language);
    let body = json!({
        "name": name,
        "path": path.to_string_lossy(),
        "language": language,
        "parser": parser,
        "role": role_for(name),
        "derives_from": null,
    });
    match client.post::<Value, Value>("/v1/fleet/repos", &body).await {
        Ok(_) => Ok(true),
        Err(e) => {
            let msg = format!("{e:#}");
            if msg.contains("409") || msg.to_lowercase().contains("already") {
                Ok(false)
            } else {
                Err(e)
            }
        }
    }
}

fn detect_language(path: &Path) -> String {
    if path.join("Cargo.toml").exists() {
        return "rust".to_string();
    }
    if path.join("package.json").exists() {
        return "typescript".to_string();
    }
    if path.join("pyproject.toml").exists() || path.join("setup.py").exists() {
        return "python".to_string();
    }
    "rust".to_string()
}

fn default_parser(language: &str) -> &'static str {
    match language {
        "rust" => "syn",
        _ => "tree-sitter",
    }
}

fn role_for(name: &str) -> &'static str {
    if name == "convergio" {
        "engine"
    } else {
        "downstream"
    }
}

fn render_build_summary(stats: &Value) {
    println!("  fleet build complete:");
    if let Some(repos) = stats.get("repos").and_then(Value::as_array) {
        for r in repos {
            let name = r.get("name").and_then(Value::as_str).unwrap_or("?");
            let files = r.get("files_embedded").and_then(Value::as_i64).unwrap_or(0);
            println!("    {:<24} {} files", name, files);
        }
    }
    if let Some(total) = stats.get("total_embeddings").and_then(Value::as_i64) {
        println!("    total embeddings: {}", total);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_language_picks_rust_for_cargo() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        assert_eq!(detect_language(dir.path()), "rust");
    }

    #[test]
    fn detect_language_picks_typescript_for_package_json() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("package.json"), "{}").unwrap();
        assert_eq!(detect_language(dir.path()), "typescript");
    }

    #[test]
    fn role_for_convergio_is_engine_others_downstream() {
        assert_eq!(role_for("convergio"), "engine");
        assert_eq!(role_for("convergio-edu"), "downstream");
        assert_eq!(role_for("foo"), "downstream");
    }

    #[test]
    fn default_parser_falls_back_to_tree_sitter() {
        assert_eq!(default_parser("rust"), "syn");
        assert_eq!(default_parser("python"), "tree-sitter");
    }
}
