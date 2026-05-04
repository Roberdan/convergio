//! Integration tests for [`convergio_tui::bus_stream`].
//!
//! Spins a tiny tokio-based HTTP/1.1 server that emits a canonical
//! `text/event-stream` response, points the supervisor at it, and
//! asserts the buffer accumulates the expected events with correct
//! seq ordering and topic-family colour cues. A second test verifies
//! the polling fallback kicks in when the SSE endpoint replies 404.

use convergio_tui::bus_stream::sse_parser::TopicFamily;
use convergio_tui::bus_stream::{spawn, Transport};
use convergio_tui::client::BusMessage;
use convergio_tui::panes::bus::message_line;
use convergio_tui::state::AppState;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::time::timeout;

/// Spin a one-shot HTTP server that, for any GET request, replies
/// with the supplied `body`. Returns the bound URL.
async fn spawn_server(body: &'static [u8], status: u16) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        loop {
            let (mut sock, _) = match listener.accept().await {
                Ok(p) => p,
                Err(_) => return,
            };
            let body = body;
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                let _ = sock.read(&mut buf).await;
                let header = match status {
                    200 => "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\n\r\n",
                    404 => "HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\nContent-Length: 9\r\n\r\nnot found",
                    _ => "HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\n\r\n",
                };
                let _ = sock.write_all(header.as_bytes()).await;
                let _ = sock.write_all(body).await;
                let _ = sock.flush().await;
                // Hold open briefly so the client reads the body
                // before the server-initiated close races the read.
                tokio::time::sleep(Duration::from_millis(150)).await;
                let _ = sock.shutdown().await;
            });
        }
    });
    url
}

#[tokio::test]
async fn supervisor_buffers_three_sse_events_with_topic_color_classes() {
    let body = b"event: bus\n\
data: {\"id\":\"m1\",\"seq\":1,\"plan_id\":\"plan-x\",\"topic\":\"coordination/agents\",\"sender\":\"alpha\",\"payload\":{\"text\":\"hello\"},\"created_at\":\"2026-05-04T19:23:45Z\"}\n\n\
event: bus\n\
data: {\"id\":\"m2\",\"seq\":2,\"plan_id\":\"plan-x\",\"topic\":\"agent.status\",\"sender\":\"beta\",\"payload\":{\"text\":\"world\"},\"created_at\":\"2026-05-04T19:23:46Z\"}\n\n\
event: bus\n\
data: {\"id\":\"m3\",\"seq\":3,\"plan_id\":\"plan-x\",\"topic\":\"system.cold-start\",\"sender\":null,\"payload\":{\"text\":\"booted\"},\"created_at\":\"2026-05-04T19:23:47Z\"}\n\n";
    let url = spawn_server(body, 200).await;
    let handle = spawn(url);
    handle.set_plan(Some("plan-x".to_string()));

    // Wait until at least 3 events arrive (or time out).
    let buffer = timeout(Duration::from_secs(5), async {
        loop {
            let snap = handle.snapshot();
            if snap.len() >= 3 {
                break snap;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("buffer should fill within 5s");
    assert_eq!(buffer.len(), 3);
    // newest first.
    assert_eq!(buffer[0].seq, 3);
    assert_eq!(buffer[1].seq, 2);
    assert_eq!(buffer[2].seq, 1);
    assert_eq!(handle.transport(), Transport::Sse);

    // Verify topic-family classification matches the brief's spec.
    assert_eq!(
        TopicFamily::classify(&buffer[2].topic),
        TopicFamily::Coordination
    );
    assert_eq!(TopicFamily::classify(&buffer[1].topic), TopicFamily::Agent);
    assert_eq!(TopicFamily::classify(&buffer[0].topic), TopicFamily::System);

    // Render via the pane and verify each topic appears.
    let line1 = message_line(&buffer[0], false);
    let plain1: String = line1.spans.iter().map(|s| s.content.as_ref()).collect();
    assert!(plain1.contains("system.cold-start"));
}

#[tokio::test]
async fn supervisor_falls_back_to_polling_on_404() {
    // 404 SSE endpoint forces the supervisor onto the polling path.
    let url = spawn_server(b"", 404).await;
    let handle = spawn(url);
    handle.set_plan(Some("plan-x".to_string()));
    let transport = timeout(Duration::from_secs(5), async {
        loop {
            match handle.transport() {
                Transport::Polling => break Transport::Polling,
                _ => tokio::time::sleep(Duration::from_millis(50)).await,
            }
        }
    })
    .await
    .expect("transport should reach Polling within 5s");
    assert_eq!(transport, Transport::Polling);
}

#[tokio::test]
async fn app_state_wires_handle_and_renders_pane_with_live_messages() {
    let body = b"event: bus\n\
data: {\"id\":\"m1\",\"seq\":42,\"plan_id\":\"plan-z\",\"topic\":\"agent:claude\",\"sender\":\"claude-code\",\"payload\":{\"text\":\"ping\"},\"created_at\":\"2026-05-04T20:00:00Z\"}\n\n";
    let url = spawn_server(body, 200).await;
    let handle = spawn(url);
    let mut state = AppState {
        bus_stream: Some(handle.clone()),
        ..AppState::default()
    };
    handle.set_plan(Some("plan-z".to_string()));
    // Drain into messages.
    timeout(Duration::from_secs(5), async {
        loop {
            state.merge_live_bus_pub();
            if !state.messages.is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("live message should appear in state.messages");
    let msg: &BusMessage = state.messages.first().unwrap();
    assert_eq!(msg.seq, 42);
    assert_eq!(msg.topic, "agent:claude");
}
