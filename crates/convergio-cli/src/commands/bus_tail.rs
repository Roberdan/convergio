//! `cvg bus tail --follow` and the `bus list` static dump.
//!
//! Streaming consumer for the SSE feed shipped by P1.1
//! (`/v1/plans/:plan_id/messages/stream`). Without `--follow`, both
//! `bus tail` and `bus list` page the existing
//! `/v1/plans/:plan_id/messages/tail` endpoint and exit. With
//! `--follow`, we stay connected to the SSE feed; on disconnect we
//! reconnect using the last-seen `seq` as the cursor; if the daemon
//! does not advertise streaming (404 on the SSE path) we fall back
//! to a 1s polling loop and warn on stderr.

use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use std::time::Duration;

use super::bus_render::{drain_events, render, BusOutput, Envelope};
use super::Client;
use convergio_i18n::Bundle;

/// Print the latest N messages and exit (no streaming).
///
/// Used for both `cvg bus list` and `cvg bus tail` without `--follow`.
pub async fn list(
    client: &Client,
    bundle: &Bundle,
    output: BusOutput,
    plan_id: &str,
    topic: Option<&str>,
    since: i64,
    limit: i64,
) -> Result<()> {
    let messages = fetch_tail(client, plan_id, topic, since, limit).await?;
    if messages.is_empty() && matches!(output, BusOutput::Human) {
        eprintln!("{}", bundle.t("bus-tail-empty", &[]));
        return Ok(());
    }
    if matches!(output, BusOutput::Human) {
        let count = messages.len().to_string();
        println!(
            "{}",
            bundle.t("bus-list-summary", &[("plan", plan_id), ("count", &count)],)
        );
    }
    for env in &messages {
        render(env, output)?;
    }
    Ok(())
}

/// Subscribe to the SSE feed and render events as they arrive.
///
/// Reconnects with `since=<last-seq>` on disconnect. On 404 (the
/// daemon does not advertise streaming) we fall back to polling
/// `/messages/tail` every second and emit a one-line stderr warning.
pub async fn follow(
    client: &Client,
    bundle: &Bundle,
    output: BusOutput,
    plan_id: &str,
    topic: Option<&str>,
    since: i64,
) -> Result<()> {
    if matches!(output, BusOutput::Human) {
        eprintln!("{}", bundle.t("bus-tail-following", &[("plan", plan_id)]));
    }
    let mut cursor = since;
    loop {
        match stream_once(client, plan_id, topic, cursor, output).await {
            Ok(last_seq) => {
                cursor = last_seq.unwrap_or(cursor);
                if matches!(output, BusOutput::Human) {
                    eprintln!("{}", bundle.t("bus-tail-disconnect", &[]));
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
            Err(StreamErr::NotStreaming) => {
                eprintln!(
                    "{}",
                    bundle.t("bus-tail-streaming-unavailable-fallback-polling", &[])
                );
                return poll_loop(client, plan_id, topic, cursor, output).await;
            }
            Err(StreamErr::Other(e)) => {
                if matches!(output, BusOutput::Human) {
                    eprintln!("{}", bundle.t("bus-tail-disconnect", &[]));
                }
                tracing::debug!(error = %e, "bus tail stream error");
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

#[derive(Debug)]
enum StreamErr {
    NotStreaming,
    Other(anyhow::Error),
}

impl<E: Into<anyhow::Error>> From<E> for StreamErr {
    fn from(e: E) -> Self {
        Self::Other(e.into())
    }
}

async fn stream_once(
    client: &Client,
    plan_id: &str,
    topic: Option<&str>,
    since: i64,
    output: BusOutput,
) -> Result<Option<i64>, StreamErr> {
    let mut url = format!(
        "{}/v1/plans/{}/messages/stream?since={since}",
        client.base(),
        plan_id
    );
    if let Some(t) = topic {
        url.push_str(&format!("&topic={t}"));
    }
    let resp = reqwest::Client::new().get(&url).send().await?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(StreamErr::NotStreaming);
    }
    if !resp.status().is_success() {
        return Err(StreamErr::Other(anyhow!(
            "stream HTTP {}: {}",
            resp.status(),
            resp.text().await.unwrap_or_default()
        )));
    }
    let mut stream = resp.bytes_stream();
    let mut buf = String::new();
    let mut last_seq: Option<i64> = None;
    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(|e| StreamErr::Other(e.into()))?;
        buf.push_str(&String::from_utf8_lossy(&bytes));
        for env in drain_events(&mut buf) {
            last_seq = Some(env.seq);
            render(&env, output).map_err(StreamErr::Other)?;
        }
    }
    Ok(last_seq)
}

async fn poll_loop(
    client: &Client,
    plan_id: &str,
    topic: Option<&str>,
    mut cursor: i64,
    output: BusOutput,
) -> Result<()> {
    let mut tick = tokio::time::interval(Duration::from_secs(1));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tick.tick().await;
        let batch = match fetch_tail(client, plan_id, topic, cursor, 100).await {
            Ok(b) => b,
            Err(_) => continue,
        };
        for env in &batch {
            cursor = cursor.max(env.seq);
            render(env, output)?;
        }
    }
}

async fn fetch_tail(
    client: &Client,
    plan_id: &str,
    topic: Option<&str>,
    since: i64,
    limit: i64,
) -> Result<Vec<Envelope>> {
    let mut path = format!("/v1/plans/{plan_id}/messages/tail?cursor={since}&limit={limit}");
    if let Some(t) = topic {
        path.push_str(&format!("&topic={t}"));
    }
    client.get::<Vec<Envelope>>(&path).await
}
