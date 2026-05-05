#!/usr/bin/env bash
# cvg-spawn.sh — render a Convergio register/heartbeat/retire wrapper
# for a Claude Code subagent brief.
#
# Pure renderer: no network I/O, no filesystem writes outside stdout.
# The register / heartbeat / retire HTTP calls fire when the subagent
# actually runs the rendered block.
#
# Reads:
#   $1                  — task description (required, used to seed the id slug)
#   $CONVERGIO_API_URL  — defaults to http://127.0.0.1:8420 (used in rendered block only)
#   $CONVERGIO_SPAWN_HEX — optional 8-char hex override for deterministic tests
#
# Writes:
#   stdout — the 6-line bash block, then a single summary line
#   stderr — usage errors only

set -euo pipefail

if [ "$#" -lt 1 ] || [ -z "${1:-}" ]; then
    echo "usage: cvg-spawn.sh <task-description>" >&2
    exit 2
fi

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

# 8 random hex chars; CONVERGIO_SPAWN_HEX is the deterministic seam
# used by the integration smoke test.
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
SUBAGENT_ID="subagent-${SLUG}-${HEX}"

# Wrapper. One logical step per line so the parent agent can paste
# it verbatim. The brief itself goes between the heartbeat-reminder
# line and the retire line. Budget pre-check (P1-6, finding E5):
# the subagent runs check-context-budget.sh before writing, targets
# ≤ 90% of the per-crate cap, and snapshots remaining budget every
# ~200 LOC of new code.
cat <<RENDER
SUBAGENT_ID="${SUBAGENT_ID}"
# P1-6 budget pre-check — abort early if any crate is over the per-crate cap.
./scripts/check-context-budget.sh || { echo "context-budget pre-check refused; pick a smaller task or refactor first" >&2; exit 1; }
curl -fsS -X POST ${DAEMON_URL}/v1/agent-registry/agents -H 'Content-Type: application/json' -d "{\"id\":\"\${SUBAGENT_ID}\",\"kind\":\"subagent\",\"name\":\"${TASK_DESC}\",\"host\":\"\${HOSTNAME:-macOS}\",\"capabilities\":[\"edit\",\"read\",\"shell\"]}" >/dev/null
curl -fsS -X POST ${DAEMON_URL}/v1/agent-registry/agents/\${SUBAGENT_ID}/heartbeat -H 'Content-Type: application/json' -d '{"status":"working"}' >/dev/null
# heartbeat every 5 min while working: re-run the line above.
# Snapshot remaining budget every ~200 LOC: ./scripts/check-context-budget.sh
# ... do work (the actual subagent brief goes here) ...
curl -fsS -X POST ${DAEMON_URL}/v1/agent-registry/agents/\${SUBAGENT_ID}/retire -H 'Content-Type: application/json' -d '{}' >/dev/null
RENDER
printf 'Subagent %s will be registered as kind=subagent\n' "${SUBAGENT_ID}"
exit 0
