#!/usr/bin/env bash
set -euo pipefail

# Convergio runner adapter: opus-overnight
#
# Intended to be spawned via POST /v1/agents/spawn-runner with kind=claude and
# command pointing at this file. The daemon will set:
#   CONVERGIO_AGENT_ID, CONVERGIO_TASK_ID, CONVERGIO_PLAN_ID
#
# This wrapper:
#   1) asks `cvg --output json agent spawn --dry-run` for the exact vendor CLI
#      argv + stdin prompt (so the logic stays in Rust)
#   2) runs the vendor CLI, forwarding stdout/stderr live
#   3) extracts token/cost telemetry from Claude stream-json output
#   4) posts `evidence.kind=usage` back to the daemon

TASK_ID="${CONVERGIO_TASK_ID:?CONVERGIO_TASK_ID is required}"
AGENT_ID="${CONVERGIO_AGENT_ID:?CONVERGIO_AGENT_ID is required}"
RUNNER_KIND="${CONVERGIO_RUNNER_KIND:-claude:opus}"
PROFILE="${CONVERGIO_PROFILE:-standard}"
# Stable label used for `usage.payload.model` when the vendor CLI does not
# surface an explicit model string.
USAGE_MODEL_FALLBACK="${CONVERGIO_USAGE_MODEL_FALLBACK:-claude-opus-overnight}"

# Build the prepared command via cvg (typed, stable contract).
PREP_FILE="$(mktemp)"
cleanup() { rm -f "${PREP_FILE}"; }
trap cleanup EXIT
CONVERGIO_NO_DRIFT_WARN=1 cvg --output json agent spawn \
  --task "${TASK_ID}" \
  --runner "${RUNNER_KIND}" \
  --agent-id "${AGENT_ID}" \
  --profile "${PROFILE}" \
  --dry-run >"${PREP_FILE}"

# Execute + capture usage using python (keeps the bash wrapper tiny).
python3 - "${TASK_ID}" "${PREP_FILE}" "${USAGE_MODEL_FALLBACK}" "${RUNNER_KIND}" <<'PY'
import json
import os
import subprocess
import sys
import threading

TASK_ID = sys.argv[1]
PREP_FILE = sys.argv[2]
USAGE_MODEL_FALLBACK = sys.argv[3]
RUNNER_KIND = sys.argv[4] if len(sys.argv) > 4 else ""

with open(PREP_FILE, "r", encoding="utf-8") as f:
    prep = json.load(f)
program = prep.get("program")
args = prep.get("args") or []
prompt = prep.get("stdin_prompt") or ""
cwd = prep.get("cwd") or None

if not program:
    print("error: cvg dry-run JSON missing 'program'", file=sys.stderr)
    sys.exit(2)

usage = {
    "input_tokens": None,
    "output_tokens": None,
    "model": (USAGE_MODEL_FALLBACK or None),
    "cost_usd": None,
}


def parse_int(val):
    if isinstance(val, bool):
        return None
    if isinstance(val, int):
        return val
    if isinstance(val, float):
        return int(val)
    if isinstance(val, str):
        s = val.strip()
        if not s:
            return None
        try:
            return int(s)
        except Exception:
            try:
                return int(float(s))
            except Exception:
                return None
    return None


def parse_float(val):
    if isinstance(val, bool):
        return None
    if isinstance(val, (int, float)):
        return float(val)
    if isinstance(val, str):
        s = val.strip()
        if not s:
            return None
        try:
            return float(s)
        except Exception:
            return None
    return None


def scan(v):
    # Heuristic: recursively walk any JSON object looking for keys named like
    # the usage payload contract.
    if isinstance(v, dict):
        for k, val in v.items():
            lk = str(k).lower()
            if lk == "input_tokens":
                n = parse_int(val)
                if n is not None and n >= 0:
                    usage["input_tokens"] = n
            elif lk == "output_tokens":
                n = parse_int(val)
                if n is not None and n >= 0:
                    usage["output_tokens"] = n
            elif lk == "cost_usd":
                n = parse_float(val)
                if n is not None and n >= 0:
                    usage["cost_usd"] = n
            elif lk == "cost_usd_micros":
                n = parse_int(val)
                if n is not None and n >= 0:
                    # Some vendor CLIs report micro-dollars; normalize to USD.
                    usage["cost_usd"] = float(n) / 1_000_000.0
            elif lk == "model" and isinstance(val, str) and val.strip():
                usage["model"] = val.strip()
            else:
                scan(val)
    elif isinstance(v, list):
        for item in v:
            scan(item)

proc = subprocess.Popen(
    [program] + list(args),
    cwd=cwd,
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=True,
    bufsize=1,
)

assert proc.stdin is not None
proc.stdin.write(prompt)
proc.stdin.close()

# Forward stderr in a thread to avoid deadlocks.
def pump_stderr():
    assert proc.stderr is not None
    for line in proc.stderr:
        sys.stderr.write(line)

stderr_t = threading.Thread(target=pump_stderr, daemon=True)
stderr_t.start()

assert proc.stdout is not None
for line in proc.stdout:
    sys.stdout.write(line)
    line = line.strip()
    if not line:
        continue
    try:
        obj = json.loads(line)
    except Exception:
        continue
    scan(obj)

code = proc.wait()

# Attach usage evidence best-effort (don't mask the vendor exit code).
if usage["input_tokens"] is not None and usage["output_tokens"] is not None and usage["model"]:
    payload = {
        "input_tokens": int(usage["input_tokens"]),
        "output_tokens": int(usage["output_tokens"]),
        "model": usage["model"],
        "cost_usd": usage["cost_usd"],
    }
    env = dict(os.environ)
    env["CONVERGIO_NO_DRIFT_WARN"] = "1"
    try:
        subprocess.run(
            [
                "cvg",
                "--output",
                "plain",
                "evidence",
                "add",
                TASK_ID,
                "--kind",
                "usage",
                "--payload",
                json.dumps(payload, separators=(",", ":")),
            ],
            env=env,
            check=False,
            text=True,
        )
    except Exception as e:
        print(f"warning: failed to attach usage evidence: {e}", file=sys.stderr)
else:
    print("warning: could not extract usage telemetry from claude output", file=sys.stderr)

sys.exit(code)
PY
