//! Build-time generator for `actions.json`.
//!
//! This is the canonical discoverable action type registry (P3-1).
//! External tools (MCP bridge, skills) can read the JSON file without
//! re-implementing the Rust catalog.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

#[path = "src/action.rs"]
#[allow(dead_code)]
mod action_surface;

use action_surface::Action;
use serde::Serialize;

#[derive(Debug, Serialize)]
struct ActionMetadataJson {
    name: String,
    capability: &'static str,
    summary: &'static str,
}

#[derive(Debug, Serialize)]
struct ActionsRegistryJson {
    schema_version: String,
    actions: Vec<ActionMetadataJson>,
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=src/action.rs");
    println!("cargo:rerun-if-changed=src/lib.rs");

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);

    let schema_version = read_schema_version(&manifest_dir.join("src/lib.rs"))?;
    let actions = Action::ALL
        .iter()
        .map(|a| ActionMetadataJson {
            name: a.as_str().to_string(),
            capability: a.capability(),
            summary: a.summary(),
        })
        .collect();

    let doc = ActionsRegistryJson {
        schema_version,
        actions,
    };

    let mut json = serde_json::to_string(&doc)?;
    json.push('\n');

    write_if_changed(&out_dir.join("actions.json"), json.as_bytes())?;
    write_if_changed(&manifest_dir.join("actions.json"), json.as_bytes())?;

    Ok(())
}

fn read_schema_version(path: &Path) -> Result<String, Box<dyn Error>> {
    let content = fs::read_to_string(path)?;
    for line in content.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("pub const SCHEMA_VERSION: &str = \"") {
            if let Some(end) = rest.find('"') {
                return Ok(rest[..end].to_string());
            }
        }
    }
    Err(format!("SCHEMA_VERSION not found in {}", path.display()).into())
}

fn write_if_changed(path: &Path, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    if let Ok(existing) = fs::read(path) {
        if existing == bytes {
            return Ok(());
        }
    }
    fs::write(path, bytes)?;
    Ok(())
}
