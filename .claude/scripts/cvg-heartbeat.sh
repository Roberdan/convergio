#!/usr/bin/env bash
# Post a Convergio agent heartbeat if more than 300s have passed
# since the last one. Called by the Claude Code PostToolUse hook.
# Best-effort: exits 0 even on daemon unavailability.
set -euo pipefail

AGENT_ID="claude-code-${USER:-anon}"
STAMP="/tmp/.cvg_hb_${USER:-anon}"
NOW=$(date +%s)
LAST=$(cat "$STAMP" 2>/dev/null || echo 0)

if [ $((NOW - LAST)) -lt 300 ]; then
    exit 0
fi

# Try cvg first; fall back to curl.
if command -v cvg >/dev/null 2>&1; then
    cvg agent heartbeat "$AGENT_ID" --status working 2>/dev/null && echo "$NOW" > "$STAMP"
else
    curl -sf -X POST \
        "http://127.0.0.1:8420/v1/agent-registry/agents/${AGENT_ID}/heartbeat" \
        -H 'Content-Type: application/json' \
        -d '{"status":"working"}' \
        >/dev/null 2>&1 && echo "$NOW" > "$STAMP"
fi

exit 0
