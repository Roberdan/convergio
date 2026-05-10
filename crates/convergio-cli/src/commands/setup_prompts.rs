//! Per-host `prompt.txt` template emitter for `cvg setup agent <host>`.
//!
//! Split from [`super::setup`] to keep the parent file under the
//! 300-line cap (CONSTITUTION § 13). The prompt template starts with
//! a "Step 0 — register your session" block so every host (Claude
//! Code, Claude Desktop, Cursor, Cline, Continue, Copilot, qwen, raw
//! shell) registers itself with the local Convergio agent registry on
//! session start. Without this, peer agents cannot see the session
//! and the daemon cannot detect liveness.

use super::setup::AgentHost;

/// Render the `prompt.txt` content for a given agent host.
///
/// The first section after the title is always "Step 0 — register
/// your session". The `agent_id` placeholder is host-shaped via
/// [`agent_id_placeholder`] so two concurrent sessions on the same
/// machine cannot collide.
pub fn prompt_snippet(host: AgentHost) -> String {
    let host_str = host.as_str();
    format!(
        "# Convergio adapter — {host_str}\n\n{step0}\n\n{rest}",
        step0 = step_zero_snippet(host),
        rest = working_loop_snippet(),
    )
}

/// The Step 0 bootstrap block injected into existing `prompt.txt`.
///
/// Keep this self-contained: it must make sense when inserted into an
/// older prompt that already has its own working loop text.
pub fn step_zero_snippet(host: AgentHost) -> String {
    let host_str = host.as_str();
    let agent_id = agent_id_placeholder(host);
    format!(
        "## Step 0 — register your session\n\n\
         When this session starts, your VERY FIRST action must be to register\n\
         yourself with the local Convergio daemon and poll your inbox. This\n\
         makes you visible to peer sessions and publishes a session-started\n\
         announcement on the bus (discoverability).\n\n\
         Run this once at session start:\n\n\
         ```\n\
         cvg session register-and-poll --agent-id \"{agent_id}\" --kind {host_str} --output human\n\
         ```\n\n\
         Debug note: `register-and-poll` hits `POST /v1/agent-registry/agents`\n\
         and `POST /v1/agent-registry/agents/:id/heartbeat`, then polls\n\
         `/v1/plans/:plan_id/messages?topic=agent:{agent_id}`.\n\n\
         If it fails, the daemon is down: `cvg service start`, then retry.\n\n\
         ## Step 0.5 — load the cold-start packet\n\n\
         After Step 0, run `cvg session resume` to load live state\n\
         (daemon health, audit chain, active plan, top pending tasks,\n\
         open PRs). For Claude Code this is already automated: the\n\
         project-level `SessionStart` hook in `.claude/settings.json`\n\
         fires both `cvg session register-and-poll` AND\n\
         `cvg session resume --output plain` before the first user\n\
         prompt, so you start with full context. Set\n\
         `CONVERGIO_NO_AUTO_RESUME=1` to skip the resume half.\n\n\
         For other hosts, run it manually once after Step 0:\n\n\
         ```\n\
         cvg session resume --output plain\n\
         ```\n",
    )
}

fn working_loop_snippet() -> &'static str {
    "## Convergio protocol\n\n\
     - All state changes go through `cvg` / `convergio.act` (the daemon owns the audit chain).\n\
     - Evidence is typed by `kind` with a small JSON payload.\n\
     - Token telemetry uses `kind=usage` with payload:\n\
       `{\"input_tokens\": 123, \"output_tokens\": 456, \"model\": \"claude:opus\", \"cost_usd\": 0.12}`\n\
       (`cost_usd` may be null).\n\
\n\
     ## Working loop\n\n\
     Use Convergio as the local source of truth. Call convergio.help once. \
     Use convergio.act for task lifecycle and evidence. If a gate refuses \
     work, fix the reason, attach new evidence, and retry submit_task. \
     Do not tell the user work is done until validate_plan returns Pass — \
     agents submit, the validator (Thor) is the only path to done (ADR-0011).\n"
}

/// Host-specific placeholder that the agent must substitute when it
/// registers in the local Convergio agent registry.
///
/// The shape encodes session locality (USER, PID, WORKSPACE…) so two
/// concurrent sessions of the same host on the same machine cannot
/// collide on `agent_id`.
pub fn agent_id_placeholder(host: AgentHost) -> &'static str {
    match host {
        AgentHost::Claude => "claude-code-${USER}",
        AgentHost::OpusOvernight => "claude-opus-overnight-${USER}-${PID}",
        AgentHost::CopilotLocal => "copilot-local-${USER}-${PID}",
        AgentHost::CopilotCloud => "copilot-cloud-${REPO_FULL_NAME}-${RUN_ID}",
        AgentHost::Cursor => "cursor-${USER}-${WORKSPACE}",
        AgentHost::Cline => "cline-${USER}",
        AgentHost::Continue => "continue-${USER}",
        AgentHost::Qwen => "qwen-${USER}",
        AgentHost::Shell => "shell-${USER}-${PPID}",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_HOSTS: &[AgentHost] = &[
        AgentHost::Claude,
        AgentHost::OpusOvernight,
        AgentHost::CopilotLocal,
        AgentHost::CopilotCloud,
        AgentHost::Cursor,
        AgentHost::Cline,
        AgentHost::Continue,
        AgentHost::Qwen,
        AgentHost::Shell,
    ];

    /// Every host's prompt.txt must begin with the title line followed
    /// by the Step 0 block within the first 6 lines, so an agent that
    /// reads the head of the file cannot skip registration.
    #[test]
    fn step_zero_is_first_section_for_every_host() {
        for &host in ALL_HOSTS {
            let body = prompt_snippet(host);
            let head_blob: String = body.lines().take(6).collect::<Vec<_>>().join("\n");
            assert!(
                head_blob.contains("Step 0"),
                "host {:?} missing 'Step 0' in head: {head_blob}",
                host.as_str()
            );
            for needle in [
                agent_id_placeholder(host),
                "register-and-poll",
                "/v1/agent-registry/agents",
                "Step 0.5",
                "cvg session resume",
                "CONVERGIO_NO_AUTO_RESUME",
            ] {
                assert!(
                    body.contains(needle),
                    "host {:?} prompt.txt missing {needle:?}",
                    host.as_str()
                );
            }
        }
    }

    #[test]
    fn agent_id_placeholders_are_unique_per_host() {
        let placeholders: Vec<&str> = ALL_HOSTS.iter().map(|&h| agent_id_placeholder(h)).collect();
        let mut sorted = placeholders.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), placeholders.len(), "duplicate placeholders");
    }
}
