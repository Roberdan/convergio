---
id: 0046
status: accepted
date: 2026-05-05
topics: [agents, bus, lifecycle, stdout, observability]
related_adrs: [0024, 0025]
touches_crates: [convergio-lifecycle, convergio-server]
last_validated: 2026-05-05
---

# 0046. Sub-agent stdout relay to the plan bus

## Status

Accepted

## Context

When the executor spawns a vendor-CLI runner (Claude Code, Copilot, shell) the
subprocess stdout was discarded (`Stdio::null()`). There was no way to observe
what the agent was actually printing during a task. Retro item H12 flagged this
as a visibility gap: operators cannot watch long-running sub-agents without
shell-level `tail -f` hacks, and the TUI dashboard had nothing to show.

The bus already exists as the coordination channel (ADR-0024, ADR-0025). The
SSE stream (`/v1/plans/:plan_id/messages/stream`) is already consumed by
`cvg dash`.

## Decision

When `convergio_lifecycle::Supervisor` is created with
`Supervisor::new_with_bus(pool, bus)` **and** the spawn spec carries a
`plan_id`, the supervisor:

1. Sets `Stdio::piped()` for the child's stdout.
2. Spawns a `tokio` task (`stdout_relay::relay`) that reads lines from the
   pipe and publishes each one to the plan bus on topic
   `agent:{process_id}:stdout`, with payload
   `{ "type": "stdout", "text": "<line>", "seq": N }`.
3. The relay task owns the pipe end and exits when stdout closes (process
   exits). The child is still dropped immediately so the OS owns the process
   lifetime.

`convergio-server` creates the supervisor via `new_with_bus`, so the live
daemon relays stdout. Tests that call `Supervisor::new(pool)` keep the old
null behaviour and are unaffected.

The `agent:<id>:stdout` topic prefix is already classified as
`TopicFamily::Agent` in `convergio-tui`, so `cvg dash` receives and renders
these messages via the existing SSE subscription without any TUI changes.

## Alternatives considered

- **HTTP polling from the CLI** — requires polling by the operator; misses
  lines between polls; bypasses the bus audit trail.
- **Write stdout to a file in the worktree** — no real-time visibility;
  files leak unless cleaned up; breaks the "single source of truth" principle.
- **Add a dedicated `/v1/agents/:id/stream` SSE route** — more HTTP surface;
  the plan bus already provides this for free.

## Consequences

- Subprocess stdout is captured line-by-line only when a plan_id is present;
  processes spawned without a plan (diagnostics, one-off shell) are unchanged.
- High-frequency stdout (e.g. a tqdm progress bar) will produce one bus message
  per line. Callers should be aware this can grow the `agent_messages` table;
  a future ADR may introduce a max-messages-per-topic or TTL.
- The `convergio-lifecycle` crate now depends on `convergio-bus` (Layer 2 →
  Layer 2 is fine per the layer table; lifecycle is Layer 3).
- Existing callers of `Supervisor::new` compile and behave identically.

## References

- Retro item H12: "sub-agent stdout stream to bus"
- Task P2-8: `8d71bdad-0d32-4d34-a938-9f39aab58e55`
- ADR-0024 (bus poll exclude-sender)
- ADR-0025 (system session events topic)
