//! `cvg capability trust` — local trust-store management (ADR-0072 §3).
//!
//! Pure filesystem operations on `~/.convergio/v3/trust-store.d/`. No
//! daemon endpoint — the daemon discovers the overlay at install time.

use super::OutputMode;
use anyhow::{anyhow, bail, Context, Result};
use chrono::Utc;
use clap::Subcommand;
use convergio_capability_registry::trust_store::{TrustStore, TrustStoreEntry};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

/// `cvg capability trust` subcommands.
#[derive(Subcommand)]
pub enum TrustCommand {
    /// List entries from the baked-in + overlay trust store.
    List,
    /// Add (or replace) a trust-store entry by copying a JSON file into
    /// the overlay directory.
    Add {
        /// Path to a JSON file containing a single `TrustStoreEntry`.
        path: PathBuf,
    },
    /// Mark an overlay entry revoked by writing a revoking sibling file.
    Revoke {
        /// `key_id` of the entry to revoke.
        key_id: String,
    },
}

/// Run a `cvg capability trust` subcommand.
pub async fn run(output: OutputMode, sub: TrustCommand) -> Result<()> {
    let overlay = overlay_dir()?;
    fs::create_dir_all(&overlay)
        .with_context(|| format!("create overlay dir {}", overlay.display()))?;
    match sub {
        TrustCommand::List => list(output, &overlay),
        TrustCommand::Add { path } => add(output, &overlay, &path),
        TrustCommand::Revoke { key_id } => revoke(output, &overlay, &key_id),
    }
}

fn overlay_dir() -> Result<PathBuf> {
    let home = std::env::var_os("HOME").ok_or_else(|| anyhow!("$HOME is not set"))?;
    Ok(PathBuf::from(home).join(".convergio/v3/trust-store.d"))
}

fn safe_filename(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn baked_in_bytes() -> &'static [u8] {
    b"[]"
}

fn list(output: OutputMode, overlay: &Path) -> Result<()> {
    let store = TrustStore::load(baked_in_bytes(), overlay).context("load trust store")?;
    let now = Utc::now();
    let active = store.active_hex_keys(now);
    let entries: Vec<&TrustStoreEntry> = store.entries().collect();
    match output {
        OutputMode::Json => {
            let rows: Vec<_> = entries
                .iter()
                .map(|e| {
                    json!({
                        "key_id": e.key_id, "owner": e.owner,
                        "valid_from": e.valid_from, "valid_until": e.valid_until,
                        "revoked": e.revoked, "algorithm": e.algorithm,
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&rows)?);
        }
        OutputMode::Plain => {
            for e in &entries {
                println!(
                    "{}\t{}\t{}\t{}\t{}",
                    e.key_id,
                    e.owner.as_deref().unwrap_or(""),
                    e.valid_from,
                    e.valid_until,
                    if e.revoked { "revoked" } else { "active" }
                );
            }
        }
        OutputMode::Human => {
            if entries.is_empty() {
                println!("trust store is empty");
                return Ok(());
            }
            println!("{:<24} {:<20} {:<14} WINDOW", "KEY_ID", "OWNER", "STATUS");
            for e in &entries {
                let status = if e.revoked {
                    "revoked"
                } else if active.iter().any(|(k, _)| k == &e.key_id) {
                    "active"
                } else {
                    "out-of-window"
                };
                println!(
                    "{:<24} {:<20} {:<14} {} → {}",
                    e.key_id,
                    e.owner.as_deref().unwrap_or("-"),
                    status,
                    e.valid_from,
                    e.valid_until
                );
            }
        }
    }
    Ok(())
}

fn add(output: OutputMode, overlay: &Path, src: &Path) -> Result<()> {
    let raw =
        fs::read_to_string(src).with_context(|| format!("read entry from {}", src.display()))?;
    let entry: TrustStoreEntry = match serde_json::from_str::<TrustStoreEntry>(&raw) {
        Ok(e) => e,
        Err(_) => {
            let arr: Vec<TrustStoreEntry> =
                serde_json::from_str(&raw).context("parse trust-store entry JSON")?;
            if arr.len() != 1 {
                bail!(
                    "trust-store add expects exactly one entry, got {}",
                    arr.len()
                );
            }
            arr.into_iter().next().unwrap()
        }
    };
    entry
        .verifying_key()
        .map_err(|err| anyhow!("invalid trust-store entry: {err}"))?;
    if entry.key_id.is_empty() {
        bail!("trust-store entry has empty key_id");
    }
    let ts = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let dest = overlay.join(format!("{ts}-{}.json", safe_filename(&entry.key_id)));
    fs::write(&dest, serde_json::to_string_pretty(&vec![entry.clone()])?)
        .with_context(|| format!("write {}", dest.display()))?;
    match output {
        OutputMode::Json => println!(
            "{}",
            json!({"added": entry.key_id, "path": dest.display().to_string()})
        ),
        OutputMode::Plain => println!("added={} path={}", entry.key_id, dest.display()),
        OutputMode::Human => println!(
            "added trust-store entry `{}` → {}",
            entry.key_id,
            dest.display()
        ),
    }
    Ok(())
}

fn revoke(output: OutputMode, overlay: &Path, key_id: &str) -> Result<()> {
    let store = TrustStore::load(baked_in_bytes(), overlay).context("load trust store")?;
    let existing = store
        .entries()
        .find(|e| e.key_id == key_id)
        .ok_or_else(|| anyhow!("no trust-store entry with key_id `{key_id}`"))?
        .clone();
    let revoked_entry = TrustStoreEntry {
        revoked: true,
        ..existing
    };
    let ts = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let dest = overlay.join(format!("{ts}-{}-revoked.json", safe_filename(key_id)));
    fs::write(&dest, serde_json::to_string_pretty(&vec![revoked_entry])?)
        .with_context(|| format!("write {}", dest.display()))?;
    match output {
        OutputMode::Json => println!(
            "{}",
            json!({"revoked": key_id, "path": dest.display().to_string()})
        ),
        OutputMode::Plain => println!("revoked={key_id} path={}", dest.display()),
        OutputMode::Human => println!(
            "revoked trust-store entry `{key_id}` via {}",
            dest.display()
        ),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use convergio_capability_registry::trust_store::fixture_entry;
    use tempfile::TempDir;

    fn fixture_now() -> chrono::DateTime<Utc> {
        "2026-06-01T00:00:00Z".parse().unwrap()
    }

    #[tokio::test]
    async fn add_then_list_shows_entry() {
        let tmp = TempDir::new().unwrap();
        let overlay = tmp.path().join("trust-store.d");
        fs::create_dir_all(&overlay).unwrap();
        let (entry, _sk) = fixture_entry("ops-2026-q1");
        let entry_path = tmp.path().join("entry.json");
        fs::write(&entry_path, serde_json::to_string_pretty(&entry).unwrap()).unwrap();
        add(OutputMode::Plain, &overlay, &entry_path).unwrap();
        assert_eq!(fs::read_dir(&overlay).unwrap().count(), 1);
        let store = TrustStore::load(b"[]", &overlay).unwrap();
        assert!(store.lookup("ops-2026-q1", fixture_now()).is_some());
    }

    #[tokio::test]
    async fn revoke_writes_revoking_sibling_and_lookup_returns_none() {
        let tmp = TempDir::new().unwrap();
        let overlay = tmp.path().join("trust-store.d");
        fs::create_dir_all(&overlay).unwrap();
        let (entry, _sk) = fixture_entry("ops-2026-q1");
        fs::write(
            overlay.join("00-base.json"),
            serde_json::to_string_pretty(&vec![entry]).unwrap(),
        )
        .unwrap();
        revoke(OutputMode::Plain, &overlay, "ops-2026-q1").unwrap();
        let store = TrustStore::load(b"[]", &overlay).unwrap();
        assert!(store.lookup("ops-2026-q1", fixture_now()).is_none());
    }

    #[tokio::test]
    async fn add_rejects_malformed_json() {
        let tmp = TempDir::new().unwrap();
        let overlay = tmp.path().join("trust-store.d");
        fs::create_dir_all(&overlay).unwrap();
        let bad = tmp.path().join("bad.json");
        fs::write(&bad, "{not json").unwrap();
        let err = add(OutputMode::Plain, &overlay, &bad).unwrap_err();
        assert!(format!("{err:#}").contains("parse trust-store entry"));
    }
}
