//! Behavioral regression tests for the audit follow-up:
//!
//! - `snapshot_core` must propagate failures from `/v1/plans` instead
//!   of pretending the daemon returned an empty plan list (audit
//!   finding #1 / #14).
//! - When `/v1/plans` succeeds but a per-plan task fetch fails, the
//!   snapshot must carry a `partial` flag so the dashboard footer
//!   renders a degraded indicator instead of a success-shaped empty
//!   (finding #2).
//! - `fetch_prs_open` / `fetch_prs_closed` must distinguish "no PRs"
//!   from "gh unavailable/failed" so the PR pane can surface the
//!   difference (finding #3).
//!
//! Each test spins up a deliberately bad endpoint so the failure
//! path is exercised. They should fail against the current code and
//! pass once the fixes land.

use convergio_tui::client::Client;
use convergio_tui::client_gh::{fetch_prs_closed, fetch_prs_open};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Spin up a one-shot HTTP listener that replies with `status` to
/// any request for the path prefix. Returns the bound base URL.
async fn spawn_http(status: u16, body: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        loop {
            let (mut sock, _) = match listener.accept().await {
                Ok(p) => p,
                Err(_) => return,
            };
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                let _ = sock.read(&mut buf).await;
                let header = format!(
                    "HTTP/1.1 {status} STATUS\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
                    body.len()
                );
                let _ = sock.write_all(header.as_bytes()).await;
                let _ = sock.flush().await;
                tokio::time::sleep(Duration::from_millis(40)).await;
                let _ = sock.shutdown().await;
            });
        }
    });
    url
}

#[tokio::test]
async fn snapshot_core_propagates_plans_fetch_failure() {
    // Daemon returns 500 for /v1/plans. The dashboard must NOT pretend
    // it got an empty list — that is the success-shaped empty mode the
    // auditor flagged.
    let url = spawn_http(500, "").await;
    let client = Client::new(url);
    let res = client.snapshot_core().await;
    assert!(
        res.is_err(),
        "snapshot_core must return Err on /v1/plans failure, got {res:?}"
    );
}

/// Server that returns a valid plan list once, then 500 on every
/// per-plan tasks fetch. Used to drive the partial-snapshot path.
async fn spawn_partial_http() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        loop {
            let (mut sock, _) = match listener.accept().await {
                Ok(p) => p,
                Err(_) => return,
            };
            tokio::spawn(async move {
                let mut buf = [0u8; 4096];
                let n = sock.read(&mut buf).await.unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]);
                let first_line = req.lines().next().unwrap_or("");
                // /v1/plans → ok with one plan; /v1/plans/<id>/tasks → 500
                let body = if first_line.contains("/v1/plans HTTP")
                    || first_line.contains("/v1/plans ")
                {
                    r#"[{"id":"p1","title":"P","status":"active","created_at":"2026-05-12T00:00:00Z","updated_at":"2026-05-12T00:00:00Z"}]"#
                } else if first_line.contains("/v1/plans/p1/tasks") {
                    ""
                } else {
                    "[]"
                };
                let status = if body.is_empty() { 500 } else { 200 };
                let header = format!(
                    "HTTP/1.1 {status} STATUS\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
                    body.len()
                );
                let _ = sock.write_all(header.as_bytes()).await;
                let _ = sock.flush().await;
                tokio::time::sleep(Duration::from_millis(40)).await;
                let _ = sock.shutdown().await;
            });
        }
    });
    url
}

#[tokio::test]
async fn snapshot_core_marks_partial_when_per_plan_tasks_fail() {
    let url = spawn_partial_http().await;
    let client = Client::new(url);
    let snap = client
        .snapshot_core()
        .await
        .expect("plans fetch should succeed; only the per-plan tasks fetch errors");
    // We accept either shape: a `partial` boolean on Snapshot, or
    // (equivalently) a non-empty `errors` list. Today's code carries
    // neither, so the assert below is the regression guard.
    assert!(
        snap.partial,
        "snapshot must flag partial when a per-plan task fetch fails"
    );
}

#[tokio::test]
async fn fetch_prs_distinguishes_gh_failure_from_empty() {
    // Forge a PATH so `gh` cannot be found: the shell-out will fail
    // to spawn. Today both `fetch_prs_open` and `fetch_prs_closed`
    // swallow that into `Ok(vec![])`. After the fix they must
    // surface the failure as `Err`.
    let saved = std::env::var_os("PATH");
    // Use an obviously empty PATH the operating system cannot find
    // a real gh in.
    std::env::set_var("PATH", "/nonexistent-bin-dir-for-convergio-tui-test");
    let open = fetch_prs_open(None).await;
    let closed = fetch_prs_closed(None).await;
    if let Some(p) = saved {
        std::env::set_var("PATH", p);
    } else {
        std::env::remove_var("PATH");
    }
    assert!(
        open.is_err(),
        "fetch_prs_open must Err when gh cannot be spawned, got Ok({:?})",
        open.ok()
    );
    assert!(
        closed.is_err(),
        "fetch_prs_closed must Err when gh cannot be spawned, got Ok({:?})",
        closed.ok()
    );
}
