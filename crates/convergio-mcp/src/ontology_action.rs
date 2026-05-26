//! Helpers for the `ontology.*` action family (ADR-0053, W1 T6).
//!
//! Kept in a sibling module to actions.rs so neither file approaches
//! the 300-line crate cap. The ontology surface is intentionally
//! read-only — all three actions are GETs against the daemon.

use crate::action_params::required_str;
use crate::bridge::Bridge;
use convergio_api::{AgentCode, AgentResponse, NextHint, SCHEMA_VERSION};
use serde_json::{json, Value};

impl Bridge {
    /// `ontology.describe` — params: `kind` ("object"|"link"), `name`.
    pub(crate) async fn ontology_describe(&self, params: Value) -> AgentResponse {
        let kind = match required_str(&params, "kind") {
            Ok(v) => v,
            Err(r) => return r,
        };
        if kind != "object" && kind != "link" {
            return crate::http::invalid("kind must be \"object\" or \"link\"".into());
        }
        let name = match required_str(&params, "name") {
            Ok(v) => v,
            Err(r) => return r,
        };
        self.get(&format!("/v1/ontology/types/{kind}/{name}")).await
    }

    /// `ontology.export` — params: `name`, `format`
    /// ("jsonschema"|"shacl"), optional `version`.
    ///
    /// Fetches the raw bytes from the daemon and surfaces them as
    /// `data.bytes_utf8` so MCP clients keep byte-identity with the
    /// HTTP surface (re-parsing through `.json::<Value>()` would
    /// reorder canonical keys).
    pub(crate) async fn ontology_export(&self, params: Value) -> AgentResponse {
        let name = match required_str(&params, "name") {
            Ok(v) => v,
            Err(r) => return r,
        };
        let format = match required_str(&params, "format") {
            Ok(v) => v,
            Err(r) => return r,
        };
        if format != "jsonschema" && format != "shacl" {
            return crate::http::invalid(
                "format must be \"jsonschema\" or \"shacl\"".into(),
            );
        }
        let mut path = format!("/v1/ontology/export/{format}/object/{name}");
        if let Some(v) = params.get("version").and_then(Value::as_u64) {
            path.push_str(&format!("?version={v}"));
        }
        let url = format!("{}{}", self.url, path);
        match self.client.get(&url).send().await {
            Ok(resp) => {
                let status = resp.status();
                let bytes = resp.bytes().await.unwrap_or_default();
                let text = String::from_utf8_lossy(&bytes).to_string();
                if status.is_success() {
                    return AgentResponse {
                        ok: true,
                        code: AgentCode::Ok,
                        message: "ontology export".into(),
                        data: Some(json!({
                            "schema_version": SCHEMA_VERSION,
                            "format": format,
                            "name": name,
                            "bytes_utf8": text,
                        })),
                        next: None,
                    };
                }
                AgentResponse {
                    ok: false,
                    code: if status.as_u16() == 404 {
                        AgentCode::NotFound
                    } else {
                        AgentCode::Error
                    },
                    message: format!("ontology export refused ({})", status.as_u16()),
                    data: Some(json!({"body": text, "status": status.as_u16()})),
                    next: None,
                }
            }
            Err(e) => AgentResponse {
                ok: false,
                code: AgentCode::DaemonUnavailable,
                message: format!("daemon unavailable: {e}"),
                data: Some(json!({"url": self.url})),
                next: Some(NextHint::StartDaemon),
            },
        }
    }
}
