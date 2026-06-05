//! Test-only connector shim implementing the protocol.

use convergio_connector::protocol::{FailureKindWire, Op, ProtocolError, Request, Response};
use convergio_connector::{DiscoverItem, Health, PullPage, SchemaHash, Watermark};
use serde_json::json;
use std::io::{self, BufRead, Write};

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    let mut pull_failures_left: u32 = std::env::var("CONVERGIO_TEST_PULL_FAILS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    for line in stdin.lock().lines() {
        let Ok(line) = line else {
            break;
        };
        let req: Request = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let _ = writeln!(
                    stdout,
                    "{}",
                    serde_json::to_string(&Response {
                        id: "?".to_string(),
                        ok: false,
                        result: None,
                        error: Some(ProtocolError {
                            kind: FailureKindWire::Fatal,
                            message: format!("bad request: {e}"),
                        }),
                    })
                    .unwrap_or_else(|_| "{}".to_string())
                );
                let _ = stdout.flush();
                continue;
            }
        };

        let resp = handle(req, &mut pull_failures_left);
        let _ = writeln!(stdout, "{}", serde_json::to_string(&resp).unwrap());
        let _ = stdout.flush();
    }
}

fn handle(req: Request, pull_failures_left: &mut u32) -> Response {
    match req.op {
        Op::Health => {
            if std::env::var("CONVERGIO_TEST_AMBIENT_SECRET").is_ok() {
                return err(req.id, FailureKindWire::Fatal, "ambient env leaked");
            }
            ok(req.id, json!(Health::Healthy))
        }
        Op::SchemaHash => {
            let ch = if std::env::var("CONVERGIO_TEST_INJECTED").is_ok() {
                "1".repeat(64)
            } else {
                "0".repeat(64)
            };
            ok(req.id, json!(SchemaHash::new_hex(ch)))
        }
        Op::Watermark => ok(req.id, json!(Some(Watermark::new("w0")))),
        Op::Discover => ok(
            req.id,
            json!([DiscoverItem {
                stream: "people".to_string(),
                label: "People".to_string(),
            }]),
        ),
        Op::Pull => {
            if *pull_failures_left > 0 {
                *pull_failures_left = pull_failures_left.saturating_sub(1);
                return err(req.id, FailureKindWire::Retryable, "transient pull failure");
            }
            ok(
                req.id,
                json!(PullPage::<serde_json::Value> {
                    records: vec![json!({"source_key": "p1", "name": "Ada"})],
                    next_watermark: Some(Watermark::new("w1")),
                    has_more: false,
                }),
            )
        }
    }
}

fn ok(id: String, v: serde_json::Value) -> Response {
    Response {
        id,
        ok: true,
        result: Some(v),
        error: None,
    }
}

fn err(id: String, kind: FailureKindWire, message: &str) -> Response {
    Response {
        id,
        ok: false,
        result: None,
        error: Some(ProtocolError {
            kind,
            message: message.to_string(),
        }),
    }
}
