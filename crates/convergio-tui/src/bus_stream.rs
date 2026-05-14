//! Live bus tail via SSE for the dashboard's Bus pane.
//!
//! Subscribes to `/v1/plans/:plan_id/messages/stream` (P1.1, ADR-0029
//! P1.3 addendum) for the currently-selected plan and pushes each
//! decoded message into a shared bounded buffer the renderer reads
//! on every frame. When the daemon does not advertise SSE support —
//! older builds or a transient 404 — the supervisor falls back to
//! polling `/messages/tail?cursor=<seq>` every second.
//!
//! No background thread, no spawned shells. The supervisor exits
//! when the handle is dropped or when the runtime shuts down.

use crate::types::BusMessage;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::watch;

pub mod sse_parser;
pub use sse_parser::drain_events;

/// Most recent bus events the buffer keeps. Older events are
/// dropped on push — the dashboard is a live tail, not an archive.
pub const BUFFER_CAP: usize = 200;

/// How often the polling fallback re-fetches `/messages/tail` when
/// SSE is unavailable.
const POLL_INTERVAL: Duration = Duration::from_secs(1);

/// How long we wait before reconnecting after an SSE error.
const RECONNECT_DELAY: Duration = Duration::from_secs(2);

/// Active transport for the live Bus tail. Used to surface a footer
/// hint when SSE isn't available.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    /// No plan selected yet — the supervisor is waiting.
    #[default]
    Idle,
    /// SSE stream is connected.
    Sse,
    /// Polling fallback is active.
    Polling,
    /// Last attempt errored; supervisor is between retries.
    Reconnecting,
}

/// Handle the dashboard uses to read the live bus buffer and steer
/// the supervisor.
#[derive(Debug, Clone)]
pub struct BusStreamHandle {
    buffer: Arc<Mutex<VecDeque<BusMessage>>>,
    transport: Arc<Mutex<Transport>>,
    plan_tx: watch::Sender<Option<String>>,
}

impl BusStreamHandle {
    /// Snapshot the buffer, newest first. Cheap clone of <= 200 rows.
    pub fn snapshot(&self) -> Vec<BusMessage> {
        let guard = match self.buffer.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.iter().rev().cloned().collect()
    }

    /// Tell the supervisor to follow `plan_id` (or to stand down when
    /// `None`). Re-subscription is debounced inside the supervisor.
    pub fn set_plan(&self, plan_id: Option<String>) {
        let _ = self.plan_tx.send_replace(plan_id);
    }

    /// Current transport state — for the footer hint.
    pub fn transport(&self) -> Transport {
        match self.transport.lock() {
            Ok(g) => *g,
            Err(p) => *p.into_inner(),
        }
    }
}

/// Spawn the supervisor task. The supervisor aborts when the handle's
/// watch sender is dropped (i.e. when the dashboard exits).
pub fn spawn(daemon_url: String) -> BusStreamHandle {
    let buffer: Arc<Mutex<VecDeque<BusMessage>>> = Arc::new(Mutex::new(VecDeque::new()));
    let transport = Arc::new(Mutex::new(Transport::Idle));
    let (plan_tx, plan_rx) = watch::channel::<Option<String>>(None);
    let handle = BusStreamHandle {
        buffer: Arc::clone(&buffer),
        transport: Arc::clone(&transport),
        plan_tx,
    };
    tokio::spawn(supervisor(daemon_url, plan_rx, buffer, transport));
    handle
}

async fn supervisor(
    daemon_url: String,
    mut plan_rx: watch::Receiver<Option<String>>,
    buffer: Arc<Mutex<VecDeque<BusMessage>>>,
    transport: Arc<Mutex<Transport>>,
) {
    // Building the reqwest client only fails on TLS init / OS resource
    // exhaustion. Loop with backoff instead of falling back to
    // `Client::new()` — that helper panics on the same init failures,
    // which would kill this supervisor task and never let the
    // dashboard recover. Surface Reconnecting between attempts so the
    // footer reflects reality.
    let client = loop {
        match reqwest::Client::builder().build() {
            Ok(c) => break c,
            Err(e) => {
                tracing::warn!(error = %e, "reqwest client init failed; retrying");
                set_transport(&transport, Transport::Reconnecting);
                tokio::time::sleep(RECONNECT_DELAY).await;
            }
        }
    };
    loop {
        let plan_id = plan_rx.borrow().clone();
        match plan_id {
            None => {
                set_transport(&transport, Transport::Idle);
                clear_buffer(&buffer);
                if plan_rx.changed().await.is_err() {
                    return;
                }
            }
            Some(id) => {
                clear_buffer(&buffer);
                let mut watch_for_change = plan_rx.clone();
                tokio::select! {
                    _ = follow(&client, &daemon_url, &id, &buffer, &transport, &mut plan_rx) => {}
                    res = watch_for_change.changed() => {
                        if res.is_err() {
                            return;
                        }
                        // Mark the receiver as seen so the next iteration
                        // of the outer loop reads the latest value.
                        let _ = plan_rx.borrow_and_update();
                    }
                }
            }
        }
    }
}

async fn follow(
    client: &reqwest::Client,
    daemon_url: &str,
    plan_id: &str,
    buffer: &Arc<Mutex<VecDeque<BusMessage>>>,
    transport: &Arc<Mutex<Transport>>,
    plan_rx: &mut watch::Receiver<Option<String>>,
) {
    let mut last_seq: i64 = 0;
    loop {
        if try_sse(
            client,
            daemon_url,
            plan_id,
            buffer,
            transport,
            &mut last_seq,
        )
        .await
        {
            set_transport(transport, Transport::Reconnecting);
        } else {
            set_transport(transport, Transport::Polling);
            poll_fallback(client, daemon_url, plan_id, buffer, &mut last_seq, plan_rx).await;
        }
        tokio::time::sleep(RECONNECT_DELAY).await;
        if plan_rx.has_changed().unwrap_or(true) {
            return;
        }
    }
}

/// Returns true if the SSE connection ran (so the next iteration
/// keeps trying SSE), false if it failed outright (so the caller
/// switches to polling fallback).
async fn try_sse(
    client: &reqwest::Client,
    daemon_url: &str,
    plan_id: &str,
    buffer: &Arc<Mutex<VecDeque<BusMessage>>>,
    transport: &Arc<Mutex<Transport>>,
    last_seq: &mut i64,
) -> bool {
    let url = format!(
        "{daemon_url}/v1/plans/{plan_id}/messages/stream?since={s}",
        s = *last_seq
    );
    let resp = match client
        .get(&url)
        .header("Accept", "text/event-stream")
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => r,
        _ => return false,
    };
    let ctype = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    if !ctype.contains("text/event-stream") {
        return false;
    }
    set_transport(transport, Transport::Sse);
    let mut response = resp;
    let mut acc = String::new();
    loop {
        match response.chunk().await {
            Ok(Some(chunk)) => {
                acc.push_str(&String::from_utf8_lossy(&chunk));
                for ev in drain_events(&mut acc) {
                    if let Some(msg) = sse_parser::parse_data_payload(&ev) {
                        if msg.seq > *last_seq {
                            *last_seq = msg.seq;
                        }
                        push_message(buffer, msg);
                    }
                }
            }
            Ok(None) | Err(_) => return true,
        }
    }
}

async fn poll_fallback(
    client: &reqwest::Client,
    daemon_url: &str,
    plan_id: &str,
    buffer: &Arc<Mutex<VecDeque<BusMessage>>>,
    last_seq: &mut i64,
    plan_rx: &mut watch::Receiver<Option<String>>,
) {
    let mut interval = tokio::time::interval(POLL_INTERVAL);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    interval.tick().await;
    for _ in 0..30 {
        if plan_rx.has_changed().unwrap_or(true) {
            return;
        }
        let url = format!(
            "{daemon_url}/v1/plans/{plan_id}/messages/tail?cursor={s}&limit=100",
            s = *last_seq
        );
        if let Ok(resp) = client.get(&url).send().await {
            if let Ok(rows) = resp.json::<Vec<BusMessage>>().await {
                for row in rows {
                    if row.seq > *last_seq {
                        *last_seq = row.seq;
                    }
                    push_message(buffer, row);
                }
            }
        }
        interval.tick().await;
    }
}

pub(crate) fn push_message(buffer: &Arc<Mutex<VecDeque<BusMessage>>>, msg: BusMessage) {
    let mut g = match buffer.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if g.iter().any(|m| m.id == msg.id || m.seq == msg.seq) {
        return;
    }
    g.push_back(msg);
    while g.len() > BUFFER_CAP {
        g.pop_front();
    }
}

fn clear_buffer(buffer: &Arc<Mutex<VecDeque<BusMessage>>>) {
    if let Ok(mut g) = buffer.lock() {
        g.clear();
    }
}

fn set_transport(slot: &Arc<Mutex<Transport>>, t: Transport) {
    if let Ok(mut g) = slot.lock() {
        *g = t;
    }
}
