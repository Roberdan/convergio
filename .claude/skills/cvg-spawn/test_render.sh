#!/usr/bin/env bash
# test_render.sh — integration smoke for cvg-spawn.sh.
#
# Runs the renderer with a fixed task description and a deterministic
# CONVERGIO_SPAWN_HEX seed, then asserts the output matches the
# golden 6-line wrapper plus summary line.

set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
SCRIPT="${HERE}/cvg-spawn.sh"

if [ ! -x "${SCRIPT}" ]; then
    echo "FAIL: ${SCRIPT} is not executable" >&2
    exit 1
fi

# Force a deterministic daemon URL so the rendered curl matches the
# golden fixture regardless of the caller's environment.
export CONVERGIO_API_URL="http://127.0.0.1:8420"
export CONVERGIO_SPAWN_HEX="deadbeef"

ACTUAL="$(bash "${SCRIPT}" "P0.3 cvg-spawn skill demo")"

# Golden block — keep in lock-step with cvg-spawn.sh.
read -r -d '' EXPECTED <<'GOLDEN' || true
SUBAGENT_ID="subagent-p0-3-cvg-spawn-skill-dem-deadbeef"
curl -fsS -X POST http://127.0.0.1:8420/v1/agent-registry/agents -H 'Content-Type: application/json' -d "{\"id\":\"${SUBAGENT_ID}\",\"kind\":\"subagent\",\"name\":\"P0.3 cvg-spawn skill demo\",\"host\":\"${HOSTNAME:-macOS}\",\"capabilities\":[\"edit\",\"read\",\"shell\"]}" >/dev/null
curl -fsS -X POST http://127.0.0.1:8420/v1/agent-registry/agents/${SUBAGENT_ID}/heartbeat -H 'Content-Type: application/json' -d '{"status":"working"}' >/dev/null
# heartbeat every 5 min while working: re-run the line above
# ... do work (the actual subagent brief goes here) ...
curl -fsS -X POST http://127.0.0.1:8420/v1/agent-registry/agents/${SUBAGENT_ID}/retire -H 'Content-Type: application/json' -d '{}' >/dev/null
Subagent subagent-p0-3-cvg-spawn-skill-dem-deadbeef will be registered as kind=subagent
GOLDEN

if [ "${ACTUAL}" != "${EXPECTED}" ]; then
    echo "FAIL: cvg-spawn render drifted from golden fixture" >&2
    diff <(printf '%s\n' "${EXPECTED}") <(printf '%s\n' "${ACTUAL}") || true
    exit 1
fi

# Bad-input arm.
if bash "${SCRIPT}" >/dev/null 2>&1; then
    echo "FAIL: missing argument should exit non-zero" >&2
    exit 1
fi

echo "ok: cvg-spawn render matches golden fixture"
