//! Local capability package installation.

use crate::ApiError;
use convergio_durability::{
    Capability, CapabilitySignatureRequest, Durability, DurabilityError, NewCapability,
    TrustedCapabilityKey,
};
use flate2::read::GzDecoder;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::{Cursor, Read};
use std::path::{Component, Path, PathBuf};
use tar::Archive;
use uuid::Uuid;

const MAX_PACKAGE_BYTES: u64 = 50 * 1024 * 1024;
const MAX_ENTRY_BYTES: u64 = 10 * 1024 * 1024;

/// Request body for local capability package installation.
#[derive(Debug, Deserialize)]
pub(crate) struct InstallFileRequest {
    pub(crate) package_path: String,
    pub(crate) signature: String,
    #[serde(default)]
    pub(crate) trusted_keys: Vec<TrustedCapabilityKey>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CapabilityManifest {
    pub(crate) name: String,
    pub(crate) version: String,
    #[serde(default)]
    platforms: Option<Vec<String>>,
}

pub(crate) struct LoadedPackage {
    pub(crate) bytes: Vec<u8>,
    pub(crate) checksum: String,
    pub(crate) manifest: CapabilityManifest,
    pub(crate) manifest_json: serde_json::Value,
}

pub(crate) async fn install_file(
    dur: &Durability,
    request: InstallFileRequest,
) -> Result<Capability, ApiError> {
    let root = default_capability_root()?;
    install_file_at_root(dur, request, &root).await
}

async fn install_file_at_root(
    dur: &Durability,
    request: InstallFileRequest,
    root: &Path,
) -> Result<Capability, ApiError> {
    let package = load_package(&request.package_path)?;
    install_loaded_package(
        dur,
        package,
        request.signature,
        request.trusted_keys,
        root,
        "local-file".into(),
    )
    .await
}

pub(crate) async fn install_package_bytes(
    dur: &Durability,
    bytes: Vec<u8>,
    signature: String,
    trusted_keys: Vec<TrustedCapabilityKey>,
    source: String,
) -> Result<Capability, ApiError> {
    let root = default_capability_root()?;
    let package = load_package_bytes(bytes)?;
    install_loaded_package(dur, package, signature, trusted_keys, &root, source).await
}

async fn install_loaded_package(
    dur: &Durability,
    package: LoadedPackage,
    signature: String,
    trusted_keys: Vec<TrustedCapabilityKey>,
    root: &Path,
    source: String,
) -> Result<Capability, ApiError> {
    validate_platform(&package.manifest)?;
    dur.verify_capability_signature(CapabilitySignatureRequest {
        name: package.manifest.name.clone(),
        version: package.manifest.version.clone(),
        checksum: package.checksum.clone(),
        manifest: package.manifest_json.clone(),
        signature: signature.clone(),
        trusted_keys,
    })
    .await?;

    let final_root = root.join(&package.manifest.name);
    if final_root.exists() {
        return invalid("capability is already installed");
    }
    let stage = root
        .join(".staging")
        .join(format!("{}-{}", package.manifest.name, Uuid::new_v4()));
    std::fs::create_dir_all(&stage).map_err(invalid_package)?;
    if let Err(err) = extract_package(&package.bytes, &stage) {
        let _ = std::fs::remove_dir_all(&stage);
        return Err(err);
    }
    std::fs::create_dir_all(root).map_err(invalid_package)?;
    std::fs::rename(&stage, &final_root).map_err(invalid_package)?;

    let registered = dur
        .register_capability(NewCapability {
            name: package.manifest.name,
            version: package.manifest.version,
            status: "installed".into(),
            source,
            root_path: Some(final_root.display().to_string()),
            manifest: package.manifest_json,
            checksum: Some(package.checksum),
            signature: Some(signature),
        })
        .await;
    match registered {
        Ok(capability) => Ok(capability),
        Err(err) => {
            let _ = std::fs::remove_dir_all(&final_root);
            Err(err.into())
        }
    }
}

fn load_package(path: &str) -> Result<LoadedPackage, ApiError> {
    let metadata = std::fs::metadata(path).map_err(invalid_package)?;
    if metadata.len() > MAX_PACKAGE_BYTES {
        return invalid("capability package is too large");
    }
    let bytes = std::fs::read(path).map_err(invalid_package)?;
    load_package_bytes(bytes)
}

pub(crate) fn load_package_bytes(bytes: Vec<u8>) -> Result<LoadedPackage, ApiError> {
    if bytes.len() as u64 > MAX_PACKAGE_BYTES {
        return invalid("capability package is too large");
    }
    let checksum = format!("sha256:{}", hex::encode(Sha256::digest(&bytes)));
    let manifest_source = read_manifest(&bytes)?;
    let manifest: CapabilityManifest = toml::from_str(&manifest_source).map_err(invalid_package)?;
    let manifest_toml: toml::Value = toml::from_str(&manifest_source).map_err(invalid_package)?;
    let manifest_json = serde_json::to_value(manifest_toml).map_err(invalid_package)?;
    Ok(LoadedPackage {
        bytes,
        checksum,
        manifest,
        manifest_json,
    })
}

fn read_manifest(bytes: &[u8]) -> Result<String, ApiError> {
    let mut archive = Archive::new(GzDecoder::new(Cursor::new(bytes)));
    for entry in archive.entries().map_err(invalid_package)? {
        let mut entry = entry.map_err(invalid_package)?;
        let rel = safe_path(&entry.path().map_err(invalid_package)?)?;
        if rel == Path::new("manifest.toml") {
            let mut manifest = String::new();
            entry
                .read_to_string(&mut manifest)
                .map_err(invalid_package)?;
            return Ok(manifest);
        }
    }
    invalid("capability package missing manifest.toml")
}

fn extract_package(bytes: &[u8], stage: &Path) -> Result<(), ApiError> {
    let mut archive = Archive::new(GzDecoder::new(Cursor::new(bytes)));
    for entry in archive.entries().map_err(invalid_package)? {
        let mut entry = entry.map_err(invalid_package)?;
        let header = entry.header().clone();
        let rel = safe_path(&entry.path().map_err(invalid_package)?)?;
        let dest = stage.join(rel);
        if header.entry_type().is_dir() {
            std::fs::create_dir_all(&dest).map_err(invalid_package)?;
        } else if header.entry_type().is_file() {
            if header.size().map_err(invalid_package)? > MAX_ENTRY_BYTES {
                return invalid("capability package entry is too large");
            }
            if header.mode().map_err(invalid_package)? & 0o6000 != 0 {
                return invalid("capability package entry uses unsafe mode bits");
            }
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent).map_err(invalid_package)?;
            }
            let mut out = std::fs::File::create(&dest).map_err(invalid_package)?;
            std::io::copy(&mut entry, &mut out).map_err(invalid_package)?;
        } else {
            return invalid("capability package contains unsupported entry type");
        }
    }
    Ok(())
}

fn safe_path(path: &Path) -> Result<PathBuf, ApiError> {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => out.push(part),
            Component::CurDir => {}
            _ => return invalid("capability package contains unsafe path"),
        }
    }
    if out.as_os_str().is_empty() {
        invalid("capability package contains empty path")
    } else {
        Ok(out)
    }
}

fn validate_platform(manifest: &CapabilityManifest) -> Result<(), ApiError> {
    let Some(platforms) = &manifest.platforms else {
        return Ok(());
    };
    let current = current_platform();
    if platforms.iter().any(|p| p == "any" || p == &current) {
        Ok(())
    } else {
        invalid(format!("capability does not support platform {current}"))
    }
}

fn current_platform() -> String {
    let os = match std::env::consts::OS {
        "macos" => "darwin",
        other => other,
    };
    let arch = match std::env::consts::ARCH {
        "aarch64" => "arm64",
        other => other,
    };
    format!("{os}-{arch}")
}

fn default_capability_root() -> Result<PathBuf, ApiError> {
    let home = std::env::var("HOME").map_err(invalid_package)?;
    Ok(PathBuf::from(home).join(".convergio/capabilities"))
}

fn invalid<T>(reason: impl Into<String>) -> Result<T, ApiError> {
    Err(DurabilityError::InvalidCapability {
        reason: reason.into(),
    }
    .into())
}

fn invalid_package(err: impl std::fmt::Display) -> ApiError {
    DurabilityError::InvalidCapability {
        reason: err.to_string(),
    }
    .into()
}
