---
name: cvg-spawn
version: 1.0.0
description: |
  Wrap Claude Code subagent briefs with auto-register + heartbeat
  boilerplate so background agents stay visible in the Convergio
  daemon. Use this when invoking the Task tool for code-mutating work
  in this repo.
allowed-tools:
  - Bash
triggers:
  - cvg-spawn
  - spawn subagent
preamble-tier: 4
---

# /cvg-spawn — wrap a subagent brief with Convergio auto-register

## Why this skill exists

Claude Code subagents (the ones launched via the parent's `Task`
tool) run in a different harness from the top-level session. The
`SessionStart` hook in `.claude/settings.json` therefore does not
fire for them — they would silently disappear from the daemon's
agent registry, which means:

- `cvg agent list` cannot see them.
- The TUI dashboard cannot colour-code them.
- `cvg coherence agents` cannot detect zombies.
- Audit cannot prove who did what.

`/cvg-spawn` solves this by emitting the canonical 3-step lifecycle
wrapper (register → work → retire) plus a heartbeat reminder. The
parent agent prepends the rendered block to every subagent brief
that contains code-mutating work in this repository.

## When to invoke

The parent (top-level) agent should invoke `/cvg-spawn` immediately
**before** calling the `Task` tool, then paste the rendered block
into the head of the subagent's brief.

You do not need to invoke `/cvg-spawn` for read-only subagents
(pure search, single-shot question answering). Invoke it whenever
the subagent will edit files, run tests, push branches, or open
PRs.

## What this skill renders

The skill produces three deterministic outputs:

1. A unique `subagent_id` of the form
   `subagent-${TASK_DESC_SLUG}-${HEX8}` where:
   - `TASK_DESC_SLUG` = first 24 chars of the task description,
     lower-cased, spaces replaced with `-`, non-alnum stripped.
   - `HEX8` = 8 random hex chars (`openssl rand -hex 4`).
2. A bash block to prepend to the subagent brief:
   - line 1: `SUBAGENT_ID="subagent-<slug>-<hex8>"`
   - line 2: **budget pre-check** — `./scripts/check-context-budget.sh`
     aborts the subagent if any crate is over the per-crate hard cap
     (P1-6 / finding E5). Pick a smaller task or refactor first.
   - line 3: register POST to `/v1/agent-registry/agents` with
     `kind=subagent`.
   - line 4: heartbeat POST to
     `/v1/agent-registry/agents/${SUBAGENT_ID}/heartbeat`.
   - line 5: `# heartbeat every 5 min while working`
   - line 6: **budget mid-flight reminder** — re-run
     `check-context-budget.sh` every ~200 LOC so the subagent does
     not push past cap on push.
   - line 7: `# ... do work ...`
   - line 8: retire POST to
     `/v1/agent-registry/agents/${SUBAGENT_ID}/retire`.
3. A one-line confirmation summary the parent prints back to the
   user before launching the subagent:
   `Subagent <id> will be registered as kind=subagent`.

## Execution

```bash
bash "$(dirname "$0")/cvg-spawn.sh" <task-description>
```

`<task-description>` is the short human-readable label the parent
will pass into the `Task` tool's `description` field. The skill
slug-ifies it to seed the subagent id.

If `CONVERGIO_SPAWN_HEX` is set (used by the integration smoke
test), the skill uses it verbatim instead of generating fresh
random bytes — that is the only deterministic seam.

## Output contract

| Stream | Content |
|---|---|
| stdout | The 6-line bash block (no leading blank line) followed by the one-line summary on its own line |
| stderr | Empty on success; one-line error on bad usage |
| exit | `0` on success, `2` on missing argument |

The skill performs **no network I/O**. It only renders text. The
register / heartbeat / retire calls fire when the subagent actually
runs the rendered block.

## Why kind=subagent

The agent registry kind field is a permissive lower-case string
(see `convergio-durability::store::agent_validation`). Existing
top-level sessions register as `kind=claude` or `kind=copilot`.
Subagents register as `kind=subagent` so:

- The TUI dashboard can render them with reduced visual weight
  (smaller indent, gray accent).
- `cvg coherence agents` can decide whether to count them as
  PR-author candidates (a subagent normally is not the PR
  author — its parent is).
- Audit rows distinguish "I, the human-driven session, did this"
  from "I, a tool-spawned helper, did this".

The list of recognised kinds lives in
[`docs/multi-agent-operating-model.md`](../../../docs/multi-agent-operating-model.md)
under "Subagent lifecycle".

## Relation to /cvg-attach

`/cvg-attach` registers the **top-level** Claude Code session at
session start (the SessionStart hook calls it). `/cvg-spawn`
generates a wrapper for **subagents** spawned mid-session via the
Task tool. The two skills do not overlap — they cover the two
distinct harnesses in which Claude Code can run.
