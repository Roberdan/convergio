//! Private data shapes for `cvg capability`.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

pub(super) fn parse_trusted_keys(values: Vec<String>) -> Result<Vec<TrustedKey>> {
    values
        .into_iter()
        .map(|value| {
            let (key_id, public_key) = value
                .split_once(':')
                .ok_or_else(|| anyhow::anyhow!("trusted key must be key_id:hex_public_key"))?;
            Ok(TrustedKey {
                key_id: key_id.into(),
                public_key: public_key.into(),
            })
        })
        .collect()
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct Capability {
    pub(super) name: String,
    pub(super) version: String,
    pub(super) status: String,
}

pub(super) struct VerifyArgs {
    pub(super) name: String,
    pub(super) version: String,
    pub(super) checksum: String,
    pub(super) manifest: PathBuf,
    pub(super) signature: String,
    pub(super) trusted_keys: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct VerifyRequest {
    pub(super) name: String,
    pub(super) version: String,
    pub(super) checksum: String,
    pub(super) manifest: Value,
    pub(super) signature: String,
    pub(super) trusted_keys: Vec<TrustedKey>,
}

#[derive(Debug, Serialize)]
pub(super) struct InstallFileRequest {
    pub(super) package_path: String,
    pub(super) signature: String,
    pub(super) trusted_keys: Vec<TrustedKey>,
}

#[derive(Debug, Serialize)]
pub(super) struct TrustedKey {
    pub(super) key_id: String,
    pub(super) public_key: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct SignatureVerification {
    pub(super) name: String,
    pub(super) version: String,
    pub(super) key_id: String,
}
