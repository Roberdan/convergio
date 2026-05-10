#!/usr/bin/env bash
# test_render.sh — integration smoke for cvg-spawn.sh.
#
# Runs the renderer with a fixed task description and a deterministic
# CONVERGIO_SPAWN_HEX seed, then asserts the output matches the golden
# wrapper plus summary line (both subagent + background modes).

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

assert_render() {
    local mode="$1"
    local actual expected id

    if [ "${mode}" = "subagent" ]; then
        actual="$(bash "${SCRIPT}" "P0.3 cvg-spawn skill demo")"
        id="subagent-p0-3-cvg-spawn-skill-dem-deadbeef"
    else
        actual="$(bash "${SCRIPT}" --mode background "P0.3 cvg-spawn skill demo")"
        id="subagent-bg-p0-3-cvg-spawn-skill-dem-deadbeef"
    fi

    read -r -d '' expected_template <<'GOLDEN' || true
SUBAGENT_ID="__ID__"
CVG_SPAWN_MODE="__MODE__"
# P1-6 budget pre-check — refuse only on hard caps (exit 1). Soft warnings (exit 2) are advisory.
set +e; ./scripts/check-context-budget.sh; rc=$?; set -e; if [ "${rc}" -eq 1 ]; then echo "context-budget hard-cap refused; pick a smaller task or refactor first" >&2; exit 1; fi
curl -fsS -X POST http://127.0.0.1:8420/v1/agent-registry/agents -H 'Content-Type: application/json' -d "{\"id\":\"${SUBAGENT_ID}\",\"kind\":\"subagent\",\"name\":\"P0.3 cvg-spawn skill demo\",\"host\":\"${HOSTNAME:-macOS}\",\"capabilities\":[\"edit\",\"read\",\"shell\"],\"metadata\":{\"spawn_mode\":\"${CVG_SPAWN_MODE}\",\"spawned_by\":\"cvg-spawn\",\"parent_agent_id\":\"${CONVERGIO_PARENT_AGENT_ID:-}\",\"task_id\":\"${CONVERGIO_TASK_ID:-}\"}}" >/dev/null
curl -fsS -X POST http://127.0.0.1:8420/v1/agent-registry/agents/${SUBAGENT_ID}/heartbeat -H 'Content-Type: application/json' -d '{"status":"working"}' >/dev/null
# heartbeat every __HB_HINT__ while working: re-run the line above.
# Snapshot remaining budget every ~200 LOC: ./scripts/check-context-budget.sh
# ... do work (the actual spawned-agent brief goes here) ...
curl -fsS -X POST http://127.0.0.1:8420/v1/agent-registry/agents/${SUBAGENT_ID}/retire -H 'Content-Type: application/json' -d '{}' >/dev/null
Subagent __ID__ will be registered as kind=subagent (mode=__MODE__)
GOLDEN

    expected="${expected_template//__ID__/${id}}"
    expected="${expected//__MODE__/${mode}}"
    if [ "${mode}" = "background" ]; then
        expected="${expected//__HB_HINT__/60 s}"
    else
        expected="${expected//__HB_HINT__/5 min}"
    fi

    if [ "${actual}" != "${expected}" ]; then
        echo "FAIL: cvg-spawn render drifted from golden fixture (mode=${mode})" >&2
        diff <(printf '%s\n' "${expected}") <(printf '%s\n' "${actual}") || true
        exit 1
    fi
}

assert_render subagent
assert_render background

# Bad-input arms.
if bash "${SCRIPT}" >/dev/null 2>&1; then
    echo "FAIL: missing argument should exit non-zero" >&2
    exit 1
fi
if bash "${SCRIPT}" --mode nope "x" >/dev/null 2>&1; then
    echo "FAIL: invalid --mode should exit non-zero" >&2
    exit 1
fi

echo "ok: cvg-spawn render matches golden fixture"
