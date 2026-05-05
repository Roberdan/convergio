# cvg-attach-cursor — Cursor rule file for Convergio

Reference integration for ADR-0045. Drop the rule file into a Cursor
project and the model will register the session, drain its bus inbox
between turns, and re-fetch the cold-start packet on demand.

## What this is

Cursor loads every file under `.cursor/rules/` once at session start
and prepends them to the system prompt. There is no per-tool-use
hook, so this integration is **best-effort, model-driven**: the
model is instructed to run `cvg session poll-since-last-turn` before
every turn. It will skip the call sometimes. The daemon is still the
source of truth — `cvg agent list` always reflects real state.

For the gold-standard real-time integration, use Claude Code's
`PreToolUse` hook (see `examples/skills/cvg-attach/`).

## Install

```bash
cp examples/skills/cvg-attach-cursor/rules.md \
  .cursor/rules/cvg-attach.md
```

Then start a Cursor session in this repo. The model will run
`cvg session register-and-poll` on its first turn and you will see
the agent in `cvg agent list --kind cursor`.

## Verify

In a separate terminal:

```bash
cvg agent list --kind cursor
# expect: cursor-<USER>-<HEX> listed with status=idle or working

cvg bus tail --topic agent:cursor-<USER>-<HEX> --follow
# send a test message from another session and verify the Cursor
# agent reads it on its next turn
```

## Known limitations

- The model can skip the `poll-since-last-turn` call. There is no
  way to force it short of inspecting the assistant's actions.
- Cursor does not fire a session-end hook. The daemon's reaper
  retires stale agents after the configured timeout (default 5
  minutes idle); manually call `cvg agent retire <id>` for
  immediate cleanup.
- `pwd | shasum | cut -c1-8` is a best-effort workspace fingerprint;
  two different folders with the same name on different machines can
  collide. For multi-machine fleets, override `--agent-id` with a
  globally unique value (CI run id, hostname-based, etc.).

For the full per-host strategy table see
`docs/adr/0045-per-host-realtime-context-push.md`.
