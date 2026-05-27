//! Integration tests for the registry fetcher layer. Lives outside the
//! `src/fetcher.rs` module so that file stays under the 300-line cap.

use convergio_capability_registry::{
    CapabilityManifest, HttpsRegistryFetcher, IndexEntry, MockFetcher, RegistryError,
    RegistryFetcher, RegistryIndex, VersionEntry,
};

#[test]
fn https_fetcher_rejects_http() {
    let err = HttpsRegistryFetcher::new("http://example.com").unwrap_err();
    assert!(matches!(err, RegistryError::InvalidUrl(_)));
}

#[test]
fn https_fetcher_rejects_file_scheme() {
    let err = HttpsRegistryFetcher::new("file:///tmp/registry").unwrap_err();
    assert!(matches!(err, RegistryError::InvalidUrl(_)));
}

#[test]
fn https_fetcher_rejects_garbage() {
    assert!(HttpsRegistryFetcher::new("").is_err());
    assert!(HttpsRegistryFetcher::new("not a url").is_err());
    assert!(HttpsRegistryFetcher::new("https://").is_err());
}

#[test]
fn https_fetcher_accepts_https() {
    let f = HttpsRegistryFetcher::new("https://registry.convergio.dev/").unwrap();
    assert_eq!(f.endpoint(), "https://registry.convergio.dev");
}

#[tokio::test]
async fn mock_fetcher_round_trip() {
    let idx = RegistryIndex {
        schema_version: "v1".into(),
        name: None,
        generated_at: None,
        entries: vec![IndexEntry {
            name: "a".into(),
            latest_version: "1.0.0".into(),
            description: None,
            keywords: vec![],
        }],
    };
    let manifest = CapabilityManifest {
        name: "a".into(),
        versions: vec![VersionEntry {
            version: "1.0.0".into(),
            bundle_sha256: "sha256:00".into(),
            published_at: None,
            notes_url: None,
        }],
        authors: vec![],
        homepage: None,
        license: None,
        signing_key_id: "k1".into(),
    };
    let m = MockFetcher::builder()
        .endpoint("mock://test")
        .index(idx.clone())
        .manifest("a", manifest.clone())
        .bundle("a", "1.0.0", b"BUNDLE".to_vec())
        .signature("a", "1.0.0", b"SIG".to_vec())
        .build();

    assert_eq!(m.endpoint(), "mock://test");
    assert_eq!(m.index().await.unwrap(), idx);
    assert_eq!(m.manifest("a").await.unwrap(), manifest);
    assert_eq!(m.bundle("a", "1.0.0").await.unwrap(), b"BUNDLE");
    assert_eq!(m.signature("a", "1.0.0").await.unwrap(), b"SIG");

    assert!(matches!(
        m.manifest("missing").await.unwrap_err(),
        RegistryError::NotFound(_)
    ));
    assert!(matches!(
        m.bundle("a", "9.9.9").await.unwrap_err(),
        RegistryError::NotFound(_)
    ));
}
