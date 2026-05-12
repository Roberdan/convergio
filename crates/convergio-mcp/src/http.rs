//! Daemon HTTP request and response mapping.

use crate::bridge::Bridge;
use convergio_api::{AgentCode, AgentResponse, NextHint, SCHEMA_VERSION};
use reqwest::{Method, StatusCode};
use serde_json::{json, Value};

impl Bridge {
    pub(crate) async fn get(&self, path: &str) -> AgentResponse {
        self.request(Method::GET, path, None).await
    }

    pub(crate) async fn post(&self, path: &str, body: Value) -> AgentResponse {
        self.request(Method::POST, path, Some(body)).await
    }

    async fn request(&self, method: Method, path: &str, body: Option<Value>) -> AgentResponse {
        let url = format!("{}{}", self.url, path);
        let builder = self.client.request(method, &url);
        let result = match body {
            Some(body) => builder.json(&body).send().await,
            None => builder.send().await,
        };
        match result {
            Ok(resp) => self.daemon_response(path, resp).await,
            Err(e) => AgentResponse {
                ok: false,
                code: AgentCode::DaemonUnavailable,
                message: format!("daemon unavailable: {e}"),
                data: Some(json!({"url": self.url})),
                next: Some(NextHint::StartDaemon),
            },
        }
    }

    async fn daemon_response(&self, path: &str, resp: reqwest::Response) -> AgentResponse {
        let status = resp.status();
        let body = match resp.json::<Value>().await {
            Ok(value) => value,
            // Surface decode failures explicitly instead of collapsing to
            // `{}`, which would let a malformed daemon response masquerade
            // as a successful empty payload.
            Err(e) => {
                return AgentResponse {
                    ok: false,
                    code: AgentCode::Error,
                    message: format!("daemon returned invalid JSON: {e}"),
                    data: Some(json!({"path": path, "status": status.as_u16()})),
                    next: None,
                };
            }
        };
        if status.is_success() {
            return ok("action completed", body, success_next(path));
        }

        let error = body.get("error").unwrap_or(&body);
        let code = error.get("code").and_then(Value::as_str).unwrap_or("error");
        let message = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("daemon error")
            .to_string();

        if status == StatusCode::CONFLICT && code == "gate_refused" {
            let refusal = json!({"path": path, "status": status.as_u16(), "error": error});
            *self.last_refusal.lock().await = Some(refusal.clone());
            return AgentResponse {
                ok: false,
                code: AgentCode::GateRefused,
                message,
                data: Some(refusal),
                next: Some(NextHint::FixAddEvidenceRetrySubmit),
            };
        }

        AgentResponse {
            ok: false,
            code: if status == StatusCode::NOT_FOUND {
                AgentCode::NotFound
            } else {
                AgentCode::Error
            },
            message,
            data: Some(json!({"status": status.as_u16(), "body": body})),
            next: None,
        }
    }
}

pub(crate) fn ok(message: &str, data: Value, next: Option<NextHint>) -> AgentResponse {
    AgentResponse {
        ok: true,
        code: AgentCode::Ok,
        message: message.into(),
        data: Some(data),
        next,
    }
}

pub(crate) fn invalid(message: String) -> AgentResponse {
    AgentResponse {
        ok: false,
        code: AgentCode::InvalidRequest,
        message,
        data: None,
        next: Some(NextHint::RefreshHelp),
    }
}

pub(crate) fn mismatch(received: String) -> AgentResponse {
    AgentResponse {
        ok: false,
        code: AgentCode::SchemaVersionMismatch,
        message: "schema version mismatch".into(),
        data: Some(json!({"expected": SCHEMA_VERSION, "received": received})),
        next: Some(NextHint::RefreshHelp),
    }
}

pub(crate) fn fallback_error(message: String) -> String {
    format!(
        r#"{{"ok":false,"code":"error","message":{}}}"#,
        json!(message)
    )
}

fn success_next(path: &str) -> Option<NextHint> {
    if path.ends_with("/transition") {
        Some(NextHint::VerifyAudit)
    } else {
        None
    }
}
