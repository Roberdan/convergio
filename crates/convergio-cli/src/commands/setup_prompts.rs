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

const DAEMON_URL: &str = "http://127.0.0.1:8420";

/// Render the `prompt.txt` content for a given agent host.
///
/// The first section after the title is always "Step 0 — register
/// your session". The `agent_id` placeholder is host-shaped via
/// [`agent_id_placeholder`] so two concurrent sessions on the same
/// machine cannot collide.
pub fn prompt_snippet(host: AgentHost) -> String {
    let host_str = host.as_str();
    let agent_id = agent_id_placeholder(host);
    format!(
        "# Convergio adapter — {host_str}\n\n\
         ## Step 0 — register your session\n\n\
         When this session starts, your VERY FIRST action must be to register\n\
         yourself with the local Convergio daemon. This makes you visible to\n\
         peer sessions and gives the daemon a heartbeat to detect liveness.\n\n\
         Run this once at session start (or invoke `cvg session register-and-poll`\n\
         when that command is available in your installed cvg version):\n\n\
         ```\n\
         curl -fsS -X POST {DAEMON_URL}/v1/agent-registry/agents \\\n  \
         -H 'Content-Type: application/json' \\\n  \
         -d '{{\"id\":\"{agent_id}\",\"kind\":\"{host_str}\",\"name\":\"<descriptive>\",\"host\":\"<machine>\",\"capabilities\":[\"...\"]}}'\n\n\
         curl -fsS -X POST {DAEMON_URL}/v1/agent-registry/agents/{agent_id}/heartbeat \\\n  \
         -H 'Content-Type: application/json' -d '{{\"status\":\"idle\"}}'\n\
         ```\n\n\
         Then poll your inbox:\n\n\
         ```\n\
         curl -s \"{DAEMON_URL}/v1/plans?status=active\" | jq -r '.[].id' | while read plan; do\n  \
         curl -s \"{DAEMON_URL}/v1/plans/$plan/messages?topic=agent:{agent_id}\"\n\
         done\n\
         ```\n\n\
         If those commands fail, the daemon is down: `cvg service start`, then retry.\n\n\
         ## Step 0.5 — load the cold-start packet\n\n\
         After register, run `cvg session resume` to load live state\n\
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
         ```\n\n\
         ## Working loop\n\n\
         Use Convergio as the local source of truth. Call convergio.help once. \
         Use convergio.act for task lifecycle and evidence. If a gate refuses \
         work, fix the reason, attach new evidence, and retry submit_task. \
         Do not tell the user work is done until validate_plan returns Pass — \
         agents submit, the validator (Thor) is the only path to done (ADR-0011).\n",
    )
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

    /// Every host's prompt.txt must begin with the title line followed
    /// by the Step 0 block within the first 6 lines, so an agent that
    /// reads the head of the file cannot skip registration.
    #[test]
    fn step_zero_is_first_section_for_every_host() {
        for host in [
            AgentHost::Claude,
            AgentHost::CopilotLocal,
            AgentHost::CopilotCloud,
            AgentHost::Cursor,
            AgentHost::Cline,
            AgentHost::Continue,
            AgentHost::Qwen,
            AgentHost::Shell,
        ] {
            let body = prompt_snippet(host);
            let head: Vec<&str> = body.lines().take(6).collect();
            let head_blob = head.join("\n");
            assert!(
                head_blob.contains("Step 0"),
                "host {:?} prompt.txt is missing 'Step 0' in head: {head_blob}",
                host.as_str()
            );
            assert!(
                body.contains(agent_id_placeholder(host)),
                "host {:?} prompt.txt is missing its agent_id placeholder",
                host.as_str()
            );
            assert!(
                body.contains("register-and-poll"),
                "host {:?} prompt.txt should reference cvg session register-and-poll",
                host.as_str()
            );
            assert!(
                body.contains("/v1/agent-registry/agents"),
                "host {:?} prompt.txt is missing the registry endpoint",
                host.as_str()
            );
            assert!(
                body.contains("Step 0.5"),
                "host {:?} prompt.txt is missing the Step 0.5 (session resume) block",
                host.as_str()
            );
            assert!(
                body.contains("cvg session resume"),
                "host {:?} prompt.txt should reference cvg session resume",
                host.as_str()
            );
            assert!(
                body.contains("CONVERGIO_NO_AUTO_RESUME"),
                "host {:?} prompt.txt should mention the CONVERGIO_NO_AUTO_RESUME escape hatch",
                host.as_str()
            );
        }
    }

    #[test]
    fn agent_id_placeholders_are_unique_per_host() {
        let placeholders = [
            agent_id_placeholder(AgentHost::Claude),
            agent_id_placeholder(AgentHost::CopilotLocal),
            agent_id_placeholder(AgentHost::CopilotCloud),
            agent_id_placeholder(AgentHost::Cursor),
            agent_id_placeholder(AgentHost::Cline),
            agent_id_placeholder(AgentHost::Continue),
            agent_id_placeholder(AgentHost::Qwen),
            agent_id_placeholder(AgentHost::Shell),
        ];
        let mut sorted = placeholders.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), placeholders.len(), "duplicate placeholders");
    }
}
