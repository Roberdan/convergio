//! Versioned Ed25519 trust store for the remote capability registry.
//!
//! Two layers, merged at load time (see ADR-0072 § 2):
//!
//! 1. **Baked-in roots** — JSON shipped inside the binary (passed to
//!    [`TrustStore::with_builtin`]).
//! 2. **Operator overlay** — every `*.json` file under the directory
//!    passed to [`TrustStore::merge_overlay_dir`], processed in
//!    lexicographic order. Later entries with the same `key_id`
//!    **replace** earlier ones — that is how operators rotate or
//!    revoke roots without rebuilding the daemon.

use crate::base64;
use crate::error::{RegistryError, Result};
use chrono::{DateTime, Utc};
use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

const ED25519_PUBLIC_KEY_LEN: usize = 32;

/// One Ed25519 trust root, as stored on disk and inside the binary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustStoreEntry {
    /// Stable identifier (e.g. `"convergio-root-2026"`). Matches
    /// [`crate::CapabilityManifest::signing_key_id`].
    pub key_id: String,

    /// Algorithm tag. Currently always `"ed25519"`.
    pub algorithm: String,

    /// Public key, base64-standard. 32 bytes once decoded.
    pub public_key_b64: String,

    /// Earliest moment at which this key is considered valid.
    pub valid_from: DateTime<Utc>,

    /// Latest moment at which this key is considered valid (exclusive).
    pub valid_until: DateTime<Utc>,

    /// Free-form display string for `cvg capability trust list`.
    #[serde(default)]
    pub owner: Option<String>,

    /// Set by the operator to disable a previously-trusted key.
    #[serde(default)]
    pub revoked: bool,
}

impl TrustStoreEntry {
    /// Decoded Ed25519 verifying key. Validates algorithm tag, key
    /// length, and base64 shape. **Does not** check revocation status
    /// or validity window — both depend on context the entry alone
    /// cannot see (revocation is decided at lookup time by
    /// [`TrustStore::lookup`], windows depend on "now" which the
    /// caller controls).
    pub fn verifying_key(&self) -> Result<VerifyingKey> {
        if !self.algorithm.eq_ignore_ascii_case("ed25519") {
            return Err(RegistryError::TrustStoreEntry(format!(
                "{}: unsupported algorithm {:?}",
                self.key_id, self.algorithm
            )));
        }
        let raw = base64::decode(&self.public_key_b64).map_err(|e| {
            RegistryError::TrustStoreEntry(format!("{}: invalid base64: {}", self.key_id, e))
        })?;
        let bytes: [u8; ED25519_PUBLIC_KEY_LEN] = raw.try_into().map_err(|raw: Vec<u8>| {
            RegistryError::TrustStoreEntry(format!(
                "{}: expected {} bytes, got {}",
                self.key_id,
                ED25519_PUBLIC_KEY_LEN,
                raw.len()
            ))
        })?;
        VerifyingKey::from_bytes(&bytes).map_err(|e| {
            RegistryError::TrustStoreEntry(format!("{}: malformed key: {}", self.key_id, e))
        })
    }
}

/// In-memory representation of the merged trust store. Cheap to clone.
#[derive(Debug, Clone, Default)]
pub struct TrustStore {
    by_id: BTreeMap<String, TrustStoreEntry>,
}

impl TrustStore {
    /// Empty trust store. Useful for tests.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Trust store seeded from baked-in JSON bytes.
    pub fn with_builtin(builtin_json: &[u8]) -> Result<Self> {
        let entries: Vec<TrustStoreEntry> = serde_json::from_slice(builtin_json)?;
        let mut store = Self::empty();
        for e in entries {
            store.insert_validated(e)?;
        }
        Ok(store)
    }

    /// Load every `*.json` file in `dir` and merge it on top of the
    /// existing entries. Files are processed in lexicographic order;
    /// later files (and entries) override earlier ones by `key_id`.
    /// `dir` not existing is **not** an error.
    pub fn merge_overlay_dir(&mut self, dir: &Path) -> Result<()> {
        if !dir.exists() {
            return Ok(());
        }
        let mut files: Vec<_> = std::fs::read_dir(dir)?
            .filter_map(|r| r.ok())
            .map(|de| de.path())
            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("json"))
            .collect();
        files.sort();
        for path in files {
            let bytes = std::fs::read(&path)?;
            let entries: Vec<TrustStoreEntry> = serde_json::from_slice(&bytes)
                .map_err(|e| RegistryError::TrustStore(format!("{}: {}", path.display(), e)))?;
            for e in entries {
                self.insert_validated(e)?;
            }
        }
        Ok(())
    }

    /// One-shot constructor: baked-in + overlay directory.
    pub fn load(builtin_json: &[u8], overlay_dir: &Path) -> Result<Self> {
        let mut store = Self::with_builtin(builtin_json)?;
        store.merge_overlay_dir(overlay_dir)?;
        Ok(store)
    }

    /// Look up a key by id, enforcing the validity window against
    /// `now`. Returns `None` when the key is unknown, revoked, or
    /// outside its window. Use [`Self::lookup_detail`] to get the
    /// reason.
    pub fn lookup(&self, key_id: &str, now: DateTime<Utc>) -> Option<&TrustStoreEntry> {
        let entry = self.by_id.get(key_id)?;
        if entry.revoked || now < entry.valid_from || now >= entry.valid_until {
            return None;
        }
        Some(entry)
    }

    /// Diagnostic variant of [`Self::lookup`].
    pub fn lookup_detail(
        &self,
        key_id: &str,
        now: DateTime<Utc>,
    ) -> std::result::Result<&TrustStoreEntry, TrustLookupRefusal> {
        let entry = self.by_id.get(key_id).ok_or(TrustLookupRefusal::Unknown)?;
        if entry.revoked {
            return Err(TrustLookupRefusal::Revoked);
        }
        if now < entry.valid_from {
            return Err(TrustLookupRefusal::NotYetValid);
        }
        if now >= entry.valid_until {
            return Err(TrustLookupRefusal::Expired);
        }
        Ok(entry)
    }

    /// Iterate over all entries (does not filter by validity window).
    pub fn entries(&self) -> impl Iterator<Item = &TrustStoreEntry> {
        self.by_id.values()
    }

    /// Bridge helper for [`convergio-durability`](../../convergio-durability)
    /// and other downstream verifiers that key off a hex-encoded
    /// public key.
    ///
    /// Yields `(key_id, public_key_hex)` for every entry that is
    /// *currently selectable* at `now` (not revoked, within its
    /// validity window). Hex output is lowercase, no `0x` prefix —
    /// matches `convergio-durability::TrustedCapabilityKey.public_key`.
    ///
    /// Revoked or out-of-window keys are silently dropped so the
    /// downstream verifier physically cannot pick them. This is the
    /// same invariant [`Self::lookup`] already enforces.
    pub fn active_hex_keys(&self, now: DateTime<Utc>) -> Vec<(String, String)> {
        self.by_id
            .values()
            .filter(|e| !e.revoked && now >= e.valid_from && now < e.valid_until)
            .filter_map(|e| {
                let raw = base64::decode(&e.public_key_b64).ok()?;
                if raw.len() != ED25519_PUBLIC_KEY_LEN {
                    return None;
                }
                Some((e.key_id.clone(), hex::encode(raw)))
            })
            .collect()
    }

    /// Number of entries (including revoked).
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// True when the store has no entries.
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    fn insert_validated(&mut self, entry: TrustStoreEntry) -> Result<()> {
        if entry.key_id.trim().is_empty() {
            return Err(RegistryError::TrustStoreEntry("empty key_id".into()));
        }
        if entry.valid_until <= entry.valid_from {
            return Err(RegistryError::TrustStoreEntry(format!(
                "{}: valid_until ({}) must be > valid_from ({})",
                entry.key_id, entry.valid_until, entry.valid_from
            )));
        }
        let _ = entry.verifying_key()?;
        self.by_id.insert(entry.key_id.clone(), entry);
        Ok(())
    }
}

/// Reason [`TrustStore::lookup_detail`] returned `Err`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustLookupRefusal {
    /// No entry with the given `key_id` exists in the store.
    Unknown,
    /// Entry exists but is flagged `revoked: true`.
    Revoked,
    /// Entry exists but `now < valid_from`.
    NotYetValid,
    /// Entry exists but `now >= valid_until`.
    Expired,
}

impl std::fmt::Display for TrustLookupRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Unknown => "unknown key_id",
            Self::Revoked => "revoked",
            Self::NotYetValid => "not yet valid",
            Self::Expired => "expired",
        };
        f.write_str(s)
    }
}

/// Build a fixture [`TrustStoreEntry`] paired with the [`ed25519_dalek::SigningKey`]
/// that owns it. Exposed unconditionally so downstream crates can pull in
/// a deterministic Ed25519 root for their own test suites — see
/// ADR-0072 § "Testing strategy".
pub fn fixture_entry(key_id: &str) -> (TrustStoreEntry, ed25519_dalek::SigningKey) {
    let mut seed = [0u8; 32];
    for (i, b) in seed.iter_mut().enumerate() {
        *b = (i as u8).wrapping_add(key_id.len() as u8);
    }
    let signing = ed25519_dalek::SigningKey::from_bytes(&seed);
    let entry = TrustStoreEntry {
        key_id: key_id.into(),
        algorithm: "ed25519".into(),
        public_key_b64: base64::encode(signing.verifying_key().as_bytes()),
        valid_from: DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
        valid_until: DateTime::parse_from_rfc3339("2027-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
        owner: Some("test".into()),
        revoked: false,
    };
    (entry, signing)
}
