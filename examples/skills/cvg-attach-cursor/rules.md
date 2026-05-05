# Convergio attach (Cursor rule file)

Drop this file into `.cursor/rules/cvg-attach.md` of any repo where
the local Convergio daemon is running on `127.0.0.1:8420`. Cursor
loads it once at session start and prepends it to the system prompt.

The rule has two responsibilities:

1. **At session start** — register this Cursor session as a
   Convergio agent so peer sessions can see it.
2. **Before every assistant turn** — drain the bus inbox so the model
   reads any messages addressed to this agent before it answers.

Cursor does not expose a per-tool-use hook today (see ADR-0045), so
this rule is best-effort: it depends on the model honoring the
instruction. The Convergio daemon stays the source of truth either
way — `cvg agent list` and `cvg session resume` always reflect
real state.

---

You are running inside a Cursor session in a repo that uses Convergio
(`http://127.0.0.1:8420`). Treat the Convergio daemon as the source
of truth for plans, tasks, evidence, and the bus.

## On the FIRST turn of a session

Run, in this order:

```bash
cvg session register-and-poll \
  --agent-id "cursor-${USER}-$(pwd | shasum | cut -c1-8)" \
  --kind cursor \
  --output plain
cvg session resume --output plain
```

The first command registers this session in the agent registry and
drains any pending direct messages on `agent:<id>`. The second prints
the cold-start packet (daemon health, audit chain, active plan, top
pending tasks, open PRs).

If `cvg` is not on PATH, fall back to:

```bash
curl -fsS -X POST http://127.0.0.1:8420/v1/agent-registry/agents \
  -H 'Content-Type: application/json' \
  -d '{"id":"cursor-'"${USER}"'-'"$(pwd | shasum | cut -c1-8)"'","kind":"cursor","name":"cursor session","host":"macOS","capabilities":["edit","bash"]}'
```

## Before EVERY assistant turn that is not the first

Run:

```bash
cvg session poll-since-last-turn \
  --agent-id "cursor-${USER}-$(pwd | shasum | cut -c1-8)" \
  --output plain
```

If the output is non-empty, treat it as a system message that
arrived between the previous turn and this one — read it, respond
to it if it is an `agent:<my-id>` direct message, and acknowledge it
with `cvg bus ack <message_id>`.

If you are about to make a non-trivial change (new file, refactor,
migration), also run:

```bash
cvg agent list --status working --kind cursor,claude --output plain
```

so you can see who else is editing this repo and avoid stomping on
their work.

## On session end

Cursor does not fire a Stop hook today. Manually run:

```bash
cvg agent retire "cursor-${USER}-$(pwd | shasum | cut -c1-8)"
```

…or rely on the daemon's reaper to retire the agent after the
configured stale window (CONSTITUTION § Reaper).

---

For the canonical Convergio agent protocol, see `AGENTS.md` and
`docs/agent-resume-packet.md`. For the reasoning behind this
rules-file approach versus Claude Code's native hooks, see
`docs/adr/0045-per-host-realtime-context-push.md`.
