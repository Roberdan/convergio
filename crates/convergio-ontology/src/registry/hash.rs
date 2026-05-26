use crate::registry::error::RegistryError;
use serde::Serialize;
use sha2::{Digest, Sha256};

/// Compute a stable SHA-256 hex digest for a JSON-serializable value.
pub fn content_hash_hex<T: Serialize>(value: &T) -> Result<String, RegistryError> {
    let bytes = serde_json::to_vec(value).map_err(RegistryError::Serialize)?;
    let digest = Sha256::digest(bytes);
    Ok(hex::encode(digest))
}
