//! README snippet emitter for `cvg setup agent <host>`.
//!
//! Split from `setup.rs` to keep files under the 300-line cap.

use super::setup::AgentHost;

pub(super) fn readme_snippet(host: AgentHost) -> String {
    let base = format!(
        "Convergio adapter: {host}\n\n\
         1. Ensure `convergio start` is running.\n\
         2. Add mcp.json to the host's MCP configuration.\n\
         3. Add prompt.txt to the agent's custom instructions.\n\
         4. Run `cvg doctor --json` if the agent cannot connect.\n",
        host = host.as_str()
    );

    if matches!(host, AgentHost::Claude) {
        return format!(
            "{base}\n\
             Extras for Claude Code (PRD-001 / Wave 0b):\n\
             5. Copy skill-cvg-attach/ into ~/.claude/skills/cvg-attach/.\n\
             6. Make cvg-attach.sh executable: chmod +x ~/.claude/skills/cvg-attach/cvg-attach.sh.\n\
             7. Merge settings.json into ~/.claude/settings.json (or the per-repo .claude/settings.json) to wire the SessionStart hook.\n\
             8. Verify with `cvg status --agents` after starting a new session.\n"
        );
    }

    if matches!(host, AgentHost::OpusOvernight) {
        return format!(
            "{base}\n\
             Opus overnight wrapper:\n\
             - run.sh is a shell adapter intended for `/v1/agents/spawn-runner` workflows.\n\
             - It expects CONVERGIO_TASK_ID + CONVERGIO_AGENT_ID and runs:\n\
               `cvg agent spawn --runner claude:opus ...`\n\
             - It attaches `evidence.kind=usage` automatically when token telemetry is available.\n"
        );
    }

    base
}
