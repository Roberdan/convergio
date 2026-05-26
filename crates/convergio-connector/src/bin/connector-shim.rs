//! Test-only connector shim implementing the protocol.

use convergio_connector::protocol::{FailureKindWire, Op, ProtocolError, Request, Response};
use convergio_connector::{DiscoverItem, Health, PullPage, SchemaHash, Watermark};
use serde_json::json;
use std::io::{self, BufRead, Write};

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
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

        let resp = handle(req);
        let _ = writeln!(stdout, "{}", serde_json::to_string(&resp).unwrap());
        let _ = stdout.flush();
    }
}

fn handle(req: Request) -> Response {
    match req.op {
        Op::Health => ok(req.id, json!(Health::Healthy)),
        Op::SchemaHash => ok(req.id, json!(SchemaHash::new_hex("0".repeat(64)))),
        Op::Watermark => ok(req.id, json!(Some(Watermark::new("w0")))),
        Op::Discover => ok(
            req.id,
            json!([DiscoverItem {
                stream: "people".to_string(),
                label: "People".to_string(),
            }]),
        ),
        Op::Pull => ok(
            req.id,
            json!(PullPage::<serde_json::Value> {
                records: vec![json!({"source_key": "p1", "name": "Ada"})],
                next_watermark: Some(Watermark::new("w1")),
                has_more: false,
            }),
        ),
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
