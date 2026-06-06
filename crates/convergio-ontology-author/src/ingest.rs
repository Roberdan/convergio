//! Document ingestion: convert source files to grounding text.
//!
//! Conversion goes through **markitdown** (never LibreOffice). The
//! [`DocConverter`] seam keeps the network/subprocess dependency
//! injectable so the pipeline is unit-testable with a deterministic
//! stub. Each converted document is hashed (sha-256 of its markdown)
//! so provenance can record exactly what grounded the ontology.

use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

use crate::error::{AuthorError, Result};

/// A converted source document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceDoc {
    /// Original path.
    pub path: PathBuf,
    /// Markdown body produced by the converter.
    pub markdown: String,
    /// Lowercase-hex sha-256 of `markdown`.
    pub content_hash: String,
}

impl SourceDoc {
    /// Build a source doc, computing the content hash from the markdown.
    pub fn new(path: PathBuf, markdown: String) -> Self {
        let content_hash = hex(&Sha256::digest(markdown.as_bytes()));
        Self {
            path,
            markdown,
            content_hash,
        }
    }
}

/// Converts a document path to markdown text.
pub trait DocConverter {
    /// Convert `path` to markdown, or fail with a reason.
    fn to_markdown(&self, path: &Path) -> Result<String>;
}

/// Real converter that shells out to the `markitdown` CLI.
pub struct MarkitdownConverter {
    /// The binary to invoke (default `markitdown`).
    pub bin: String,
}

impl Default for MarkitdownConverter {
    fn default() -> Self {
        Self {
            bin: "markitdown".to_string(),
        }
    }
}

impl DocConverter for MarkitdownConverter {
    fn to_markdown(&self, path: &Path) -> Result<String> {
        let output =
            Command::new(&self.bin)
                .arg(path)
                .output()
                .map_err(|e| AuthorError::DocConversion {
                    path: path.to_path_buf(),
                    reason: format!("could not run '{}': {e}", self.bin),
                })?;
        if !output.status.success() {
            return Err(AuthorError::DocConversion {
                path: path.to_path_buf(),
                reason: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

/// Convert every document in order, returning source docs.
pub fn ingest_all(converter: &dyn DocConverter, paths: &[PathBuf]) -> Result<Vec<SourceDoc>> {
    let mut docs = Vec::with_capacity(paths.len());
    for path in paths {
        let md = converter.to_markdown(path)?;
        docs.push(SourceDoc::new(path.clone(), md));
    }
    Ok(docs)
}

fn hex(bytes: &[u8]) -> String {
    const H: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(H[(b >> 4) as usize] as char);
        s.push(H[(b & 0x0f) as usize] as char);
    }
    s
}

#[cfg(test)]
pub(crate) mod testing {
    use super::*;
    use std::collections::HashMap;

    /// Deterministic converter for tests: returns canned markdown.
    #[derive(Default)]
    pub struct StubConverter {
        /// Map of path -> markdown.
        pub bodies: HashMap<PathBuf, String>,
    }

    impl DocConverter for StubConverter {
        fn to_markdown(&self, path: &Path) -> Result<String> {
            self.bodies
                .get(path)
                .cloned()
                .ok_or_else(|| AuthorError::DocConversion {
                    path: path.to_path_buf(),
                    reason: "no stub body".into(),
                })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_doc_hash_is_stable() {
        let a = SourceDoc::new(PathBuf::from("a.md"), "hello".into());
        let b = SourceDoc::new(PathBuf::from("b.md"), "hello".into());
        assert_eq!(a.content_hash, b.content_hash);
        assert_eq!(a.content_hash.len(), 64);
    }
}
