//! Wire-format types for the documents served by a registry endpoint.
//!
//! These mirror the URL scheme defined in
//! [ADR-0072 § 1](../../../docs/adr/0072-remote-capability-registry.md):
//!
//! - `GET /v1/index.json` → [`RegistryIndex`]
//! - `GET /v1/<name>/manifest.json` → [`CapabilityManifest`]
//!
//! Validation rules are kept *lenient* on optional fields and *strict* on
//! anything the verifier (F2) will rely on (`name`, `version`, `signing_key_id`).

use serde::{Deserialize, Serialize};

/// Flat catalog returned by `/v1/index.json`. Small (<1 MB at v1.0 scale)
/// and meant to be cached locally for client-side search.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryIndex {
    /// Schema version. Currently always `"v1"`.
    pub schema_version: String,

    /// Human-readable name of the registry (operator-controlled).
    #[serde(default)]
    pub name: Option<String>,

    /// ISO-8601 timestamp the index was generated at.
    #[serde(default)]
    pub generated_at: Option<String>,

    /// One row per published capability.
    #[serde(default)]
    pub entries: Vec<IndexEntry>,
}

/// One row in [`RegistryIndex::entries`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexEntry {
    /// Capability name (matches the `name` field of the `.cap` manifest).
    pub name: String,
    /// Latest version available at this registry, in semver form.
    pub latest_version: String,
    /// Short, single-line description shown by `cvg capability search`.
    #[serde(default)]
    pub description: Option<String>,
    /// Free-form keywords used to filter `cvg capability search`.
    #[serde(default)]
    pub keywords: Vec<String>,
}

/// Per-capability metadata returned by `/v1/<name>/manifest.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityManifest {
    /// Capability name. **Must** match the manifest field inside the `.cap` bundle.
    pub name: String,
    /// All versions ever published at this registry.
    pub versions: Vec<VersionEntry>,
    /// Author / contributor display strings (free-form).
    #[serde(default)]
    pub authors: Vec<String>,
    /// Project home page URL.
    #[serde(default)]
    pub homepage: Option<String>,
    /// SPDX license identifier.
    #[serde(default)]
    pub license: Option<String>,
    /// Stable trust-store identifier (matches [`crate::TrustStoreEntry::key_id`]).
    pub signing_key_id: String,
}

/// One published version inside a [`CapabilityManifest`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionEntry {
    /// Semver-style version string. Opaque to the registry layer.
    pub version: String,
    /// `sha256:<64 hex>` of the `.cap` bundle bytes.
    pub bundle_sha256: String,
    /// Optional release timestamp (ISO-8601).
    #[serde(default)]
    pub published_at: Option<String>,
    /// Optional release notes URL.
    #[serde(default)]
    pub notes_url: Option<String>,
}

impl RegistryIndex {
    /// Look up an entry by capability name.
    pub fn find(&self, name: &str) -> Option<&IndexEntry> {
        self.entries.iter().find(|e| e.name == name)
    }
}

impl CapabilityManifest {
    /// Look up a published [`VersionEntry`] by version string.
    pub fn version(&self, version: &str) -> Option<&VersionEntry> {
        self.versions.iter().find(|v| v.version == version)
    }

    /// Latest entry by appearance order (registries publish chronologically).
    pub fn latest(&self) -> Option<&VersionEntry> {
        self.versions.last()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_round_trips() {
        let idx = RegistryIndex {
            schema_version: "v1".into(),
            name: Some("convergio-registry".into()),
            generated_at: Some("2026-05-27T11:00:00Z".into()),
            entries: vec![IndexEntry {
                name: "a11y.axe".into(),
                latest_version: "1.0.0".into(),
                description: Some("Run axe-core checks".into()),
                keywords: vec!["a11y".into(), "wcag".into()],
            }],
        };

        let s = serde_json::to_string(&idx).unwrap();
        let back: RegistryIndex = serde_json::from_str(&s).unwrap();
        assert_eq!(idx, back);
        assert_eq!(back.find("a11y.axe").unwrap().latest_version, "1.0.0");
        assert!(back.find("missing").is_none());
    }

    #[test]
    fn manifest_lookup() {
        let m = CapabilityManifest {
            name: "a11y.axe".into(),
            versions: vec![
                VersionEntry {
                    version: "0.9.0".into(),
                    bundle_sha256: "sha256:aa".into(),
                    published_at: None,
                    notes_url: None,
                },
                VersionEntry {
                    version: "1.0.0".into(),
                    bundle_sha256: "sha256:bb".into(),
                    published_at: None,
                    notes_url: None,
                },
            ],
            authors: vec!["core".into()],
            homepage: None,
            license: Some("MIT".into()),
            signing_key_id: "convergio-root-2026".into(),
        };
        assert_eq!(m.version("0.9.0").unwrap().bundle_sha256, "sha256:aa");
        assert_eq!(m.latest().unwrap().version, "1.0.0");
        assert!(m.version("9.9.9").is_none());
    }

    #[test]
    fn manifest_tolerates_missing_optional_fields() {
        let raw = r#"{
            "name": "x",
            "versions": [],
            "signing_key_id": "k"
        }"#;
        let m: CapabilityManifest = serde_json::from_str(raw).unwrap();
        assert!(m.authors.is_empty());
        assert!(m.license.is_none());
    }
}
