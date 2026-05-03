//! Build the canonical embeddable text for a node and compute its
//! source hash.
//!
//! ADR-0038 § 5.4: the re-embed trigger is `source_hash` change, not
//! mtime. The hash is SHA-256 over the trimmed embeddable text.

use sha2::{Digest, Sha256};

/// Canonical embeddable text together with its source hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceText {
    /// The text that will be passed to [`crate::Embedder::embed`].
    /// Trimmed of leading/trailing whitespace.
    pub text: String,
    /// SHA-256 of [`Self::text`], hex-encoded. Matches the
    /// `graph_node_embeddings.source_hash` column verbatim.
    pub source_hash: String,
}

impl SourceText {
    /// Build a [`SourceText`] from the supplied raw text.
    ///
    /// Leading and trailing whitespace are trimmed before hashing
    /// because formatter touches must not invalidate cached
    /// embeddings (ADR-0038 § 5.4).
    pub fn new(raw: impl AsRef<str>) -> Self {
        let trimmed = raw.as_ref().trim().to_owned();
        let mut hasher = Sha256::new();
        hasher.update(trimmed.as_bytes());
        let source_hash = hex::encode(hasher.finalize());
        Self {
            text: trimmed,
            source_hash,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SourceText;

    #[test]
    fn same_input_same_hash() {
        let a = SourceText::new("hello world");
        let b = SourceText::new("hello world");
        assert_eq!(a.source_hash, b.source_hash);
        assert_eq!(a.text, b.text);
    }

    #[test]
    fn different_input_different_hash() {
        let a = SourceText::new("hello world");
        let b = SourceText::new("hello there");
        assert_ne!(a.source_hash, b.source_hash);
    }

    #[test]
    fn trims_outer_whitespace_but_keeps_inner() {
        let a = SourceText::new("  hi friend  ");
        assert_eq!(a.text, "hi friend");
    }

    #[test]
    fn formatter_touch_does_not_change_hash() {
        let a = SourceText::new("payload");
        let b = SourceText::new("\n  payload\n");
        assert_eq!(a.source_hash, b.source_hash);
    }

    #[test]
    fn hash_is_64_hex_chars() {
        let a = SourceText::new("x");
        assert_eq!(a.source_hash.len(), 64);
        assert!(a.source_hash.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
