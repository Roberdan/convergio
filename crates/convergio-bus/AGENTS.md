# AGENTS.md — convergio-bus

For repo-wide rules see [../../AGENTS.md](../../AGENTS.md).

This crate is the persisted communication channel for agents. It is how
agents coordinate without private, unaudited side chats.

## Invariants

- Messages are scoped to a plan.
- Delivery is at-least-once; consumers must be idempotent.
- Message ordering is per `(plan_id, topic)` sequence.
- Do not use the bus for hidden task state; durable state belongs in the
  owning layer.
- Future MCP bus actions must route through the daemon, not direct DB
  access.

## Crate stats

The block below is rewritten by `cvg docs regenerate` (ADR-0015) —
do not edit between the markers.

<!-- BEGIN AUTO:crate_stats -->
**`convergio-bus` stats:** 7 `*.rs` files / 13 public items / 661 lines (under `src/`).

No files within 50 lines of the 300-line cap.
<!-- END AUTO -->
