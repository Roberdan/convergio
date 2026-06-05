//! Remote capability package installation through ADR-0072 registries.

use crate::capability_install::{install_package_bytes, load_package_bytes};
use crate::ApiError;
use chrono::Utc;
use convergio_capability_registry::{HttpsRegistryFetcher, RegistryFetcher, TrustStore};
use convergio_durability::{Capability, Durability, DurabilityError, TrustedCapabilityKey};
use serde::Deserialize;

/// Request body for `POST /v1/capabilities/install-file` remote mode.
#[derive(Debug, Deserialize)]
pub(crate) struct InstallRemoteRequest {
    pub(crate) registry_url: String,
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) version: Option<String>,
}

pub(crate) async fn install_remote(
    dur: &Durability,
    request: InstallRemoteRequest,
) -> Result<Capability, ApiError> {
    let fetcher = HttpsRegistryFetcher::new(&request.registry_url).map_err(invalid_registry)?;
    install_remote_with_fetcher(dur, request, &fetcher).await
}

pub(crate) async fn install_remote_with_fetcher(
    dur: &Durability,
    request: InstallRemoteRequest,
    fetcher: &dyn RegistryFetcher,
) -> Result<Capability, ApiError> {
    let manifest = fetcher
        .manifest(&request.name)
        .await
        .map_err(invalid_registry)?;
    if manifest.name != request.name {
        return invalid(format!(
            "registry manifest name mismatch: expected {}, got {}",
            request.name, manifest.name
        ));
    }
    let version = match request.version {
        Some(v) => v,
        None => manifest
            .latest()
            .ok_or_else(|| invalid_err("registry manifest has no versions"))?
            .version
            .clone(),
    };
    let version_row = manifest
        .version(&version)
        .ok_or_else(|| invalid_err(format!("registry has no version {version}")))?;
    let bundle = fetcher
        .bundle(&request.name, &version)
        .await
        .map_err(invalid_registry)?;
    let package = load_package_bytes(bundle.clone())?;
    if package.checksum != version_row.bundle_sha256 {
        return invalid("remote bundle checksum does not match registry manifest");
    }
    if package.manifest.name != request.name || package.manifest.version != version {
        return invalid("remote bundle manifest does not match registry manifest");
    }
    let signature = signature_hex(
        fetcher
            .signature(&request.name, &version)
            .await
            .map_err(invalid_registry)?,
    )?;
    let trusted_keys = trust_keys_for(&manifest.signing_key_id)?;
    if trusted_keys.is_empty() {
        return invalid(format!(
            "no active trust-store key for {}",
            manifest.signing_key_id
        ));
    }
    install_package_bytes(
        dur,
        bundle,
        signature,
        trusted_keys,
        format!("remote-registry:{}", fetcher.endpoint()),
    )
    .await
}

fn trust_keys_for(key_id: &str) -> Result<Vec<TrustedCapabilityKey>, ApiError> {
    let home = std::env::var_os("HOME").ok_or_else(|| invalid_err("$HOME is not set"))?;
    let overlay = std::path::PathBuf::from(home).join(".convergio/v3/trust-store.d");
    let store = TrustStore::load(b"[]", &overlay).map_err(invalid_registry)?;
    Ok(store
        .active_hex_keys(Utc::now())
        .into_iter()
        .filter(|(id, _)| id == key_id)
        .map(|(key_id, public_key)| TrustedCapabilityKey { key_id, public_key })
        .collect())
}

fn signature_hex(bytes: Vec<u8>) -> Result<String, ApiError> {
    if bytes.len() == 64 {
        return Ok(hex::encode(bytes));
    }
    let s =
        String::from_utf8(bytes).map_err(|_| invalid_err("signature is not hex or raw bytes"))?;
    let trimmed = s.trim();
    if trimmed.len() == 128 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        Ok(trimmed.to_string())
    } else {
        invalid("signature is not a 64-byte value or 128-char hex string")
    }
}

fn invalid_registry(err: impl std::fmt::Display) -> ApiError {
    invalid_err(format!("remote registry: {err}"))
}

fn invalid<T>(reason: impl Into<String>) -> Result<T, ApiError> {
    Err(invalid_err(reason))
}

fn invalid_err(reason: impl Into<String>) -> ApiError {
    DurabilityError::InvalidCapability {
        reason: reason.into(),
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use convergio_capability_registry::{
        manifest::{CapabilityManifest as RemoteManifest, VersionEntry},
        trust_store::fixture_entry,
        MockFetcher,
    };
    use convergio_db::Pool;
    use convergio_durability::{capability_signature_payload, init, CapabilitySignatureRequest};
    use ed25519_dalek::Signer;
    use flate2::{write::GzEncoder, Compression};
    use sha2::{Digest, Sha256};
    use std::fs;
    use std::io::Write;
    use tar::{Builder, Header};
    use uuid::Uuid;

    #[tokio::test]
    async fn remote_install_uses_fetcher_trust_store_and_audits_row() {
        let scratch = scratch_dir();
        let home = scratch.join("home");
        fs::create_dir_all(home.join(".convergio/v3/trust-store.d")).unwrap();
        std::env::set_var("HOME", &home);
        let db_path = scratch.join("state.db");
        let pool = Pool::connect(&format!("sqlite://{}", db_path.display()))
            .await
            .unwrap_or_else(|_| panic!("remote install should succeed"));
        init(&pool).await.unwrap();
        let dur = Durability::new(pool);
        let bundle = package_bytes("planner", "0.1.0");
        let checksum = format!("sha256:{}", hex::encode(Sha256::digest(&bundle)));
        let (entry, signing) = fixture_entry("test-root");
        fs::write(
            home.join(".convergio/v3/trust-store.d/00-root.json"),
            serde_json::to_string_pretty(&vec![entry]).unwrap(),
        )
        .unwrap();
        let signature = sign_bundle(&bundle, &checksum, &signing);
        let fetcher = MockFetcher::builder()
            .endpoint("mock://registry")
            .manifest("planner", remote_manifest(&checksum))
            .bundle("planner", "0.1.0", bundle)
            .signature("planner", "0.1.0", hex::decode(signature).unwrap())
            .build();

        let cap = install_remote_with_fetcher(
            &dur,
            InstallRemoteRequest {
                registry_url: "https://registry.example".into(),
                name: "planner".into(),
                version: None,
            },
            &fetcher,
        )
        .await
        .unwrap_or_else(|_| panic!("remote install should succeed"));

        assert_eq!(cap.name, "planner");
        assert_eq!(cap.source, "remote-registry:mock://registry");
        assert!(home
            .join(".convergio/capabilities/planner/manifest.toml")
            .is_file());
        let _ = fs::remove_dir_all(scratch);
    }

    fn scratch_dir() -> std::path::PathBuf {
        let dir = std::env::current_dir()
            .unwrap()
            .join(".claude/test-scratch")
            .join(Uuid::new_v4().to_string());
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn remote_manifest(checksum: &str) -> RemoteManifest {
        RemoteManifest {
            name: "planner".into(),
            versions: vec![VersionEntry {
                version: "0.1.0".into(),
                bundle_sha256: checksum.into(),
                published_at: None,
                notes_url: None,
            }],
            authors: vec!["test".into()],
            homepage: None,
            license: Some("MIT".into()),
            signing_key_id: "test-root".into(),
        }
    }

    fn package_bytes(name: &str, version: &str) -> Vec<u8> {
        let manifest =
            format!("name = \"{name}\"\nversion = \"{version}\"\nplatforms = [\"any\"]\n");
        let mut bytes = Vec::new();
        {
            let encoder = GzEncoder::new(&mut bytes, Compression::default());
            let mut tar = Builder::new(encoder);
            append(&mut tar, "manifest.toml", manifest.as_bytes());
            append(&mut tar, "bin/planner", b"#!/bin/sh\n");
            tar.finish().unwrap();
            tar.into_inner().unwrap().finish().unwrap();
        }
        bytes
    }

    fn append<W: Write>(tar: &mut Builder<W>, path: &str, bytes: &[u8]) {
        let mut header = Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append_data(&mut header, path, bytes).unwrap();
    }

    fn sign_bundle(bundle: &[u8], checksum: &str, signing: &ed25519_dalek::SigningKey) -> String {
        let manifest = load_package_bytes(bundle.to_vec())
            .unwrap_or_else(|_| panic!("package bytes should parse"))
            .manifest_json;
        let request = CapabilitySignatureRequest {
            name: "planner".into(),
            version: "0.1.0".into(),
            checksum: checksum.into(),
            manifest,
            signature: String::new(),
            trusted_keys: vec![],
        };
        let payload = capability_signature_payload(&request).unwrap();
        hex::encode(signing.sign(&payload).to_bytes())
    }
}
