//! Phase orchestration for [`crate::handshake::run_check`].
//!
//! Each of the six phases of the handshake (register, ping, pong,
//! receive, acks, retire) is driven here. On the first failure
//! [`run_phases`] returns `Err(next_phase_to_skip_from)` so the
//! caller can pad the report with `Skipped` phases — keeping the
//! report length stable for CI consumers.

use crate::handshake::{phase_helper, Phase, PhaseOutcome};
use crate::handshake_http::{
    ack_pair, heartbeat_pair, poll_for_seq, publish, register_pair, retire_pair, BusMessage,
    PhaseFail,
};
use serde_json::json;
use std::time::{Duration, Instant};

/// Drive phases 1–6 in order. Returns `Err(start_skip_phase)` on the
/// first failure so [`crate::handshake::skip_remaining`] can pad
/// the rest as `Skipped`.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_phases(
    client: &reqwest::Client,
    daemon: &str,
    plan_id: &str,
    topic: &str,
    agent_a: &str,
    agent_b: &str,
    nonce: &str,
    timeout: Duration,
    phases: &mut Vec<Phase>,
) -> Result<(), u8> {
    // Phase 1 — register A + B and immediately heartbeat both. The
    // heartbeat call is bundled into phase 1 (rather than a separate
    // phase) so the report shape stays at six phases for CI
    // consumers, but exercising it here ensures a regression in the
    // /heartbeat route surfaces as a phase-1 fail instead of a
    // false-green E2E (Codex review on PR #197).
    let p1 = Instant::now();
    match register_pair(client, daemon, agent_a, agent_b).await {
        Ok(()) => {}
        Err(e) => {
            phases.push(phase_helper(
                1,
                "register",
                PhaseOutcome::Failed,
                p1,
                &format!("{e}"),
            ));
            return Err(2);
        }
    }
    match heartbeat_pair(client, daemon, agent_a, agent_b).await {
        Ok(()) => phases.push(phase_helper(
            1,
            "register",
            PhaseOutcome::Ok,
            p1,
            "2 agents registered + heartbeated",
        )),
        Err(e) => {
            phases.push(phase_helper(
                1,
                "register",
                PhaseOutcome::Failed,
                p1,
                &format!("heartbeat: {e}"),
            ));
            return Err(2);
        }
    }

    // Phase 2 — A publishes ping.
    let p2 = Instant::now();
    let ping_payload = json!({"type": "ping", "from": agent_a, "nonce": nonce});
    let ping = match publish(client, daemon, plan_id, topic, agent_a, &ping_payload).await {
        Ok(m) => {
            phases.push(phase_helper(
                2,
                "ping",
                PhaseOutcome::Ok,
                p2,
                &format!("published seq {}", m.seq),
            ));
            m
        }
        Err(e) => {
            phases.push(phase_helper(
                2,
                "ping",
                PhaseOutcome::Failed,
                p2,
                &format!("{e}"),
            ));
            return Err(3);
        }
    };

    // Phase 3 — B receives ping, then pongs.
    let p3 = Instant::now();
    let pong = match receive_and_pong(
        client, daemon, plan_id, topic, agent_b, &ping, nonce, timeout,
    )
    .await
    {
        Ok((seen_ms, p)) => {
            phases.push(phase_helper(
                3,
                "pong",
                PhaseOutcome::Ok,
                p3,
                &format!(
                    "seq {} received in {seen_ms}ms; pong seq {}",
                    ping.seq, p.seq
                ),
            ));
            p
        }
        Err(PhaseFail::Timeout(d)) => {
            phases.push(phase_helper(3, "pong", PhaseOutcome::Timeout, p3, &d));
            return Err(4);
        }
        Err(PhaseFail::Other(d)) => {
            phases.push(phase_helper(3, "pong", PhaseOutcome::Failed, p3, &d));
            return Err(4);
        }
    };

    // Phase 4 — A receives pong with matching nonce.
    let p4 = Instant::now();
    match poll_for_seq(client, daemon, plan_id, topic, ping.seq, agent_a, timeout).await {
        Ok(msg) => {
            if crate::handshake::validate_pong_payload(&msg.payload, nonce, &ping.id) {
                phases.push(phase_helper(
                    4,
                    "receive",
                    PhaseOutcome::Ok,
                    p4,
                    &format!("seq {} received; nonce matches", pong.seq),
                ));
            } else {
                phases.push(phase_helper(
                    4,
                    "receive",
                    PhaseOutcome::Failed,
                    p4,
                    "pong nonce mismatch",
                ));
                return Err(5);
            }
        }
        Err(PhaseFail::Timeout(d)) => {
            phases.push(phase_helper(4, "receive", PhaseOutcome::Timeout, p4, &d));
            return Err(5);
        }
        Err(PhaseFail::Other(d)) => {
            phases.push(phase_helper(4, "receive", PhaseOutcome::Failed, p4, &d));
            return Err(5);
        }
    }

    // Phase 5 — both ack their received messages.
    let p5 = Instant::now();
    match ack_pair(client, daemon, &pong.id, agent_a, &ping.id, agent_b).await {
        Ok(()) => phases.push(phase_helper(
            5,
            "acks",
            PhaseOutcome::Ok,
            p5,
            "A acked pong; B acked ping",
        )),
        Err(e) => {
            phases.push(phase_helper(
                5,
                "acks",
                PhaseOutcome::Failed,
                p5,
                &format!("{e}"),
            ));
            return Err(6);
        }
    }

    // Phase 6 — retire both agents.
    let p6 = Instant::now();
    match retire_pair(client, daemon, agent_a, agent_b).await {
        Ok(()) => phases.push(phase_helper(
            6,
            "retire",
            PhaseOutcome::Ok,
            p6,
            "2 agents retired",
        )),
        Err(e) => {
            phases.push(phase_helper(
                6,
                "retire",
                PhaseOutcome::Failed,
                p6,
                &format!("{e}"),
            ));
            return Err(7);
        }
    }
    Ok(())
}

/// Phase 3 sub-step: B sees the ping, then publishes the pong.
#[allow(clippy::too_many_arguments)]
async fn receive_and_pong(
    client: &reqwest::Client,
    daemon: &str,
    plan_id: &str,
    topic: &str,
    agent_b: &str,
    ping: &BusMessage,
    nonce: &str,
    timeout: Duration,
) -> Result<(u128, BusMessage), PhaseFail> {
    let recv_started = Instant::now();
    // Use cursor=ping.seq-1 so we are sure to see the ping itself.
    let _ping_seen = poll_for_seq(
        client,
        daemon,
        plan_id,
        topic,
        ping.seq - 1,
        agent_b,
        timeout,
    )
    .await?;
    let receive_ms = recv_started.elapsed().as_millis();
    let pong_payload =
        json!({"type": "pong", "from": agent_b, "nonce": nonce, "replying_to": ping.id});
    let pong = publish(client, daemon, plan_id, topic, agent_b, &pong_payload)
        .await
        .map_err(|e| PhaseFail::Other(format!("pong publish: {e}")))?;
    Ok((receive_ms, pong))
}
