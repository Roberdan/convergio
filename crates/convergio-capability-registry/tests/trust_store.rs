//! Integration tests for the trust store. Driven through the public
//! API plus [`fixture_entry`] so we never duplicate the seed-handling
//! logic in tests.

use chrono::{DateTime, TimeZone, Utc};
use convergio_capability_registry::trust_store::{
    fixture_entry, TrustLookupRefusal, TrustStore, TrustStoreEntry,
};
use convergio_capability_registry::RegistryError;
use std::fs;
use tempfile::tempdir;

fn now_in_window() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0).unwrap()
}

fn serialize(entries: &[&TrustStoreEntry]) -> Vec<u8> {
    serde_json::to_vec_pretty(&entries).unwrap()
}

#[test]
fn builtin_loads_and_lookup_succeeds_in_window() {
    let (e, _) = fixture_entry("root-a");
    let json = serialize(&[&e]);
    let store = TrustStore::with_builtin(&json).unwrap();

    assert_eq!(store.len(), 1);
    assert!(!store.is_empty());

    let hit = store.lookup("root-a", now_in_window()).unwrap();
    assert_eq!(hit.key_id, "root-a");
}

#[test]
fn lookup_rejects_outside_window() {
    let (e, _) = fixture_entry("root-a");
    let store = TrustStore::with_builtin(&serialize(&[&e])).unwrap();

    let before = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
    let after = Utc.with_ymd_and_hms(2027, 6, 1, 0, 0, 0).unwrap();

    assert!(store.lookup("root-a", before).is_none());
    assert!(store.lookup("root-a", after).is_none());

    assert_eq!(
        store.lookup_detail("root-a", before).unwrap_err(),
        TrustLookupRefusal::NotYetValid
    );
    assert_eq!(
        store.lookup_detail("root-a", after).unwrap_err(),
        TrustLookupRefusal::Expired
    );
}

#[test]
fn lookup_unknown_key_returns_unknown() {
    let store = TrustStore::empty();
    assert!(store.lookup("nope", now_in_window()).is_none());
    assert_eq!(
        store.lookup_detail("nope", now_in_window()).unwrap_err(),
        TrustLookupRefusal::Unknown
    );
}

#[test]
fn revoked_entry_rejected_at_load() {
    let (mut e, _) = fixture_entry("root-a");
    e.revoked = true;
    let json = serialize(&[&e]);
    let err = TrustStore::with_builtin(&json).unwrap_err();
    assert!(matches!(err, RegistryError::TrustStoreEntry(_)));
}

#[test]
fn rejects_unsupported_algorithm() {
    let (mut e, _) = fixture_entry("root-a");
    e.algorithm = "rsa".into();
    let err = TrustStore::with_builtin(&serialize(&[&e])).unwrap_err();
    assert!(matches!(err, RegistryError::TrustStoreEntry(_)));
}

#[test]
fn rejects_bad_base64() {
    let (mut e, _) = fixture_entry("root-a");
    e.public_key_b64 = "not!valid!b64".into();
    let err = TrustStore::with_builtin(&serialize(&[&e])).unwrap_err();
    assert!(matches!(err, RegistryError::TrustStoreEntry(_)));
}

#[test]
fn rejects_invalid_window() {
    let (mut e, _) = fixture_entry("root-a");
    e.valid_until = e.valid_from;
    let err = TrustStore::with_builtin(&serialize(&[&e])).unwrap_err();
    assert!(matches!(err, RegistryError::TrustStoreEntry(_)));
}

#[test]
fn overlay_overrides_by_key_id_in_lex_order() {
    let (base, _) = fixture_entry("root-a");
    let mut overridden = base.clone();
    overridden.owner = Some("operator-overlay".into());

    let dir = tempdir().unwrap();
    // `00-base.json` loads first, `99-overlay.json` overrides root-a.
    fs::write(dir.path().join("00-base.json"), serialize(&[&base])).unwrap();
    fs::write(
        dir.path().join("99-overlay.json"),
        serialize(&[&overridden]),
    )
    .unwrap();

    let store = TrustStore::load(b"[]", dir.path()).unwrap();
    let hit = store.lookup("root-a", now_in_window()).unwrap();
    assert_eq!(hit.owner.as_deref(), Some("operator-overlay"));
}

#[test]
fn missing_overlay_dir_is_ok() {
    let (e, _) = fixture_entry("root-a");
    let store = TrustStore::load(
        &serialize(&[&e]),
        std::path::Path::new("/definitely/does/not/exist/convergio"),
    )
    .unwrap();
    assert_eq!(store.len(), 1);
}

#[test]
fn entry_verifying_key_signs_and_verifies() {
    let (entry, signing) = fixture_entry("root-a");
    let vk = entry.verifying_key().unwrap();

    use ed25519_dalek::{Signer, Verifier};
    let msg = b"convergio-w9-f1";
    let sig = signing.sign(msg);
    assert!(vk.verify(msg, &sig).is_ok());

    // Tampered message must fail.
    let bad = b"convergio-w9-X1";
    assert!(vk.verify(bad, &sig).is_err());
}
