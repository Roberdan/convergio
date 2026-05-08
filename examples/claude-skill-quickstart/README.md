# claude-skill-quickstart

The smallest possible reproduction of *what Convergio actually does*:
an agent submits a piece of work, Convergio refuses it when the
evidence shows the work is broken, and accepts it when the evidence
is clean. Every refusal lands in a hash-chained audit log you can
verify from outside the daemon.

This example does **not** require a real LLM. It uses two shell
scripts to play the part of a "dirty" and a "clean" agent so you
can run the whole demo in 60 seconds. The same pattern applies to
any agent runner (Claude Code, Codex CLI, Cursor, your own
Python loop) — the agent talks to the daemon over plain HTTP via
`cvg`, and Convergio is the source of truth.

## What you see

```
without Convergio:                     with Convergio:

agent says "done" --> human believes   agent says "done" --> Convergio gates
                                                          --> 409 gate_refused
                                                              + audit row
                                       agent fixes evidence
                                                          --> 201 submitted
                                       human runs validate
                                                          --> verdict pass
                                                          --> done (set only by Thor)
                                                              + audit chain ok
```

## Prerequisites

- A running Convergio daemon at `http://127.0.0.1:8420`.

  Install (recommended, prebuilt):

  ```bash
  curl -fsSL https://raw.githubusercontent.com/Roberdan/convergio/main/scripts/install.sh | sh
  ```

  Or build from source (from the repo root):

  ```bash
  sh scripts/install-local.sh
  ```

  Then run:

  ```bash
  cvg setup
  cvg service install
  cvg service start
  cvg health
  ```

- `jq` (for the demo scripts to read JSON). On macOS:
  `brew install jq`. On Debian/Ubuntu: `apt install jq`.

## Run the demo

From this directory:

```bash
./demo-without-convergio.sh    # the world today: nobody says no
./demo-with-convergio.sh       # the world Convergio wants: refusal is loud and audited
```

Each script is ~60 lines of bash, no surprises. Read them top to
bottom — they are the documentation.

## Expected output (with-Convergio path)

The script tells two stories side by side. Plan A submits dirty work
and is refused. Plan B submits clean work and is promoted by Thor.

```
[A1] cvg plan create  (Plan A — will refuse dirty work)
[A2] cvg task create  (requires code + test evidence)
[A3] agent attaches DIRTY evidence
      diff:  "// TODO: wire this later\nfn handler() {}"
[A4] cvg task transition submitted  -> 409 gate_refused
      response: {"error":{"code":"gate_refused",
                "message":"no_debt: debt markers found in evidence:
                          code#todo_marker"}}
      [audit row written: task.refused]
      retired the task to failed (audit row preserved)

[B1] cvg plan create  (Plan B — clean attempt)
[B2] cvg task create  (same shape, fresh plan)
[B3] agent attaches CLEAN evidence
      diff:  "fn handler() -> i32 { 42 }"
[B4] cvg validate <plan> -> verdict pass
      task status: done    (set only by Thor — ADR-0011)

audit chain: N entries, ok=true
```

Two plans rather than two tasks on the same plan because Convergio's
evidence store is **append-only** by design — once dirty evidence is
attached, the task carries it forever. The audit chain integrity is
the reason: we cannot rewrite history just because we want a clean
retry. The canonical recovery is "fail the task, start a fresh one."

The audit chain entry at the refusal step is non-falsifiable: anyone
running `cvg audit verify` against this same SQLite database will see
the same hash chain, including the row where Convergio said no.

## Try it with a real agent

Once the demo runs cleanly, point your real agent runner at the
daemon and stop attaching evidence by hand. Each adapter Convergio
ships generates the boilerplate for one popular host:

```bash
cvg setup agent claude        # Claude Code / Claude Desktop MCP
cvg setup agent cursor
cvg setup agent cline
cvg setup agent continue
cvg setup agent qwen
cvg setup agent shell         # generic shell-script agent
cvg setup agent copilot-local # GitHub Copilot local IDE bridge
```

The MCP bridge exposes only two tools: `convergio.help` and
`convergio.act`. `act` accepts a small typed action vocabulary
(create_task, claim_task, add_evidence, submit_task, validate_plan,
audit_verify, ...). Agents that do not speak MCP can hit the HTTP
API directly — see [ARCHITECTURE.md](../../ARCHITECTURE.md) for the
endpoint inventory.

## What to read next

- [CONSTITUTION.md](../../CONSTITUTION.md) — the five sacred
  principles the daemon enforces.
- [docs/adr/0011-thor-only-done.md](../../docs/adr/0011-thor-only-done.md)
  — why agents cannot self-promote to `done`.
- [docs/adr/0012-ooda-aware-validation.md](../../docs/adr/0012-ooda-aware-validation.md)
  — where Thor is going (smart validation, learnings, agent ↔ Thor
  negotiation, OODA loop).
- [docs/plans/v0.1.x-friction-log.md](../../docs/plans/v0.1.x-friction-log.md)
  — every UX gap the v0.1.x dogfood session found, and what closed
  them.
