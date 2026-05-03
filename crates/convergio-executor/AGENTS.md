# AGENTS.md — convergio-executor

For repo-wide rules see [../../AGENTS.md](../../AGENTS.md).

This crate is reference dispatch behavior. It should not become the full
orchestrator for every real agent runner.

## Invariants

- Keep deterministic behavior easy to test.
- Real Claude/Copilot/Cursor runner support should be adapter/capability
  work, not hardcoded here.
- Dispatch must respect task claim state and future leases.
- Do not mark tasks done; workers must submit evidence and pass gates.

## Crate stats

The block below is rewritten by `cvg docs regenerate` (ADR-0015) —
do not edit between the markers.

<!-- BEGIN AUTO:crate_stats -->
**`convergio-executor` stats:** 4 `*.rs` files / 15 public items / 393 lines (under `src/`).

Files approaching the 300-line cap:
- `src/executor.rs` (260 lines)
<!-- END AUTO -->
