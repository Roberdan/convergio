#!/usr/bin/env bash
# cvg-spawn.sh — render a Convergio register/heartbeat/retire wrapper
# for a spawned worker brief (subagent or background agent).
#
# Pure renderer: no network I/O, no filesystem writes outside stdout.
# The register / heartbeat / retire HTTP calls fire when the spawned
# agent actually runs the rendered block.
#
# Usage:
#   bash cvg-spawn.sh [--mode subagent|background] <task-description>
#
# Reads:
#   $CONVERGIO_API_URL   — defaults to http://127.0.0.1:8420 (rendered only)
#   $CONVERGIO_SPAWN_HEX — optional 8-char hex override for deterministic tests
#
# Writes:
#   stdout — the wrapper block (no leading blank line), then a summary line
#   stderr — usage errors only

set -euo pipefail

MODE="subagent"
if [ "${1:-}" = "--mode" ]; then
    MODE="${2:-}"
    shift 2
fi

if [ "$#" -lt 1 ] || [ -z "${1:-}" ]; then
    echo "usage: cvg-spawn.sh [--mode subagent|background] <task-description>" >&2
    exit 2
fi

case "${MODE}" in
    subagent | background) ;;
    *)
        echo "error: --mode must be 'subagent' or 'background'" >&2
        exit 2
        ;;
esac

TASK_DESC="$1"
DAEMON_URL="${CONVERGIO_API_URL:-http://127.0.0.1:8420}"

# Slugify: lower-case, spaces -> '-', drop everything that is not
# [a-z0-9-], collapse repeats, trim leading/trailing '-', cap 24 chars.
slugify() {
    local s
    s="$(printf '%s' "$1" | LC_ALL=C tr '[:upper:] ' '[:lower:]-')"
    s="$(printf '%s' "$s" | LC_ALL=C tr -c 'a-z0-9-' '-')"
    s="$(printf '%s' "$s" | LC_ALL=C sed -e 's/-\{2,\}/-/g' -e 's/^-//' -e 's/-$//')"
    printf '%s' "${s:0:24}"
}

# 8 random hex chars; CONVERGIO_SPAWN_HEX is the deterministic seam.
random_hex() {
    if [ -n "${CONVERGIO_SPAWN_HEX:-}" ]; then
        printf '%s' "${CONVERGIO_SPAWN_HEX}"
    elif command -v openssl >/dev/null 2>&1; then
        openssl rand -hex 4
    elif [ -r /dev/urandom ]; then
        od -A n -t x1 -N 4 /dev/urandom | LC_ALL=C tr -d ' \n'
    else
        printf '%08x' "$(date +%s)"
    fi
}

SLUG="$(slugify "${TASK_DESC}")"
if [ -z "${SLUG}" ]; then
    SLUG="task"
fi
HEX="$(random_hex)"

# Keep kind=subagent (documented registry kind); encode mode into the id
# and metadata so `cvg agent list` stays readable.
if [ "${MODE}" = "background" ]; then
    SUBAGENT_ID="subagent-bg-${SLUG}-${HEX}"
else
    SUBAGENT_ID="subagent-${SLUG}-${HEX}"
fi

if [ "${MODE}" = "background" ]; then
    HEARTBEAT_HINT="60 s"
else
    HEARTBEAT_HINT="5 min"
fi

# Wrapper. One logical step per line so the parent agent can paste it
# verbatim. The brief itself goes between the heartbeat reminder and
# the retire line.
#
# Budget pre-check (P1-6): allow exit=2 (soft warnings), refuse only on exit=1.
cat <<RENDER
SUBAGENT_ID="${SUBAGENT_ID}"
CVG_SPAWN_MODE="${MODE}"
# P1-6 budget pre-check — refuse only on hard caps (exit 1). Soft warnings (exit 2) are advisory.
set +e; ./scripts/check-context-budget.sh; rc=\$?; set -e; if [ "\${rc}" -eq 1 ]; then echo "context-budget hard-cap refused; pick a smaller task or refactor first" >&2; exit 1; fi
curl -fsS -X POST ${DAEMON_URL}/v1/agent-registry/agents -H 'Content-Type: application/json' -d "{\"id\":\"\${SUBAGENT_ID}\",\"kind\":\"subagent\",\"name\":\"${TASK_DESC}\",\"host\":\"\${HOSTNAME:-macOS}\",\"capabilities\":[\"edit\",\"read\",\"shell\"],\"metadata\":{\"spawn_mode\":\"\${CVG_SPAWN_MODE}\",\"spawned_by\":\"cvg-spawn\",\"parent_agent_id\":\"\${CONVERGIO_PARENT_AGENT_ID:-}\",\"task_id\":\"\${CONVERGIO_TASK_ID:-}\"}}" >/dev/null
curl -fsS -X POST ${DAEMON_URL}/v1/agent-registry/agents/\${SUBAGENT_ID}/heartbeat -H 'Content-Type: application/json' -d '{"status":"working"}' >/dev/null
# heartbeat every ${HEARTBEAT_HINT} while working: re-run the line above.
# Snapshot remaining budget every ~200 LOC: ./scripts/check-context-budget.sh
# ... do work (the actual spawned-agent brief goes here) ...
curl -fsS -X POST ${DAEMON_URL}/v1/agent-registry/agents/\${SUBAGENT_ID}/retire -H 'Content-Type: application/json' -d '{}' >/dev/null
RENDER
printf 'Subagent %s will be registered as kind=subagent (mode=%s)\n' "${SUBAGENT_ID}" "${MODE}"
exit 0
