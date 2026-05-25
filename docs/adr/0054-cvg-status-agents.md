---
status: accepted
date: 2026-05-25
deciders: roberdan
---

# 0054 — `cvg status --agents` live agent listing

## Context

W5 of the production-ready plan asks for the dual surface around the
agent registry: hook scripts that auto-register sessions and a
listing verb so operators can see "who's alive" at any moment.

The hooks side ships separately; this ADR covers the listing verb.

## Decision

Add `--agents` to `cvg status`. When set:

- The plans pipeline is bypassed entirely (no `/v1/status` call).
- We call `GET /v1/agent-registry/agents` and render `agent_id`,
  `kind`, last-heartbeat age, and `leases_held` in a stable table.
- Three render modes honored: `human` (table), `plain` (TSV), `json`
  (raw daemon body — forward-compatible with envelope changes).

The body parser accepts both a bare `[…]` array and a
`{ "agents": […], …}` envelope so the route can later grow pagination
metadata without breaking older `cvg` binaries.

## Consequences

Positive:

- Operators get a one-liner answer to "is the daemon seeing my
  sessions?" without `curl`-ing.
- Sets up W5's hook follow-up: once hooks register on `SessionStart`,
  the table immediately shows results.
- Heartbeat age is rendered locally so the daemon does not need to
  pre-compute it.

Trade-offs:

- The `--agents` flag rides on `cvg status` rather than a new
  subcommand. Justification: keeps the operator's hot-path command
  surface small, mirrors the `--all`, `--show-waves`, `--mine` pattern
  already on this command.

## References

- ADR-0009 (agent registry)
- `crates/convergio-cli/src/commands/status_agents.rs`
- W5 in `convergio-production-ready-plan.md`
