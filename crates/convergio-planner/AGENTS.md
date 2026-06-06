# AGENTS.md — convergio-planner

For repo-wide rules see [../../AGENTS.md](../../AGENTS.md).

This crate is the reference planner. It proves the loop; it is not the
final AI planning brain.

## Invariants

- Keep output deterministic and testable (the heuristic backend
  is the reference for this — the Opus backend layers an LLM on
  top).
- Plans/tasks should be small enough for workers to understand.
- The reference Opus backend may name vendor CLIs and recommend
  specific provider/model combinations in its prompt (ADR-0036);
  any *additional* provider-specific planning logic must ship as
  a capability rather than landing in this crate.
- Future advanced planners should be capabilities unless they are core
  coordination logic.

## Crate stats

The block below is rewritten by `cvg docs regenerate` (ADR-0015) —
do not edit between the markers.

<!-- BEGIN AUTO:crate_stats -->
**`convergio-planner` stats:** 10 `*.rs` files / 30 public items / 1285 lines (under `src/`).

Files approaching the 300-line cap:
- `src/opus.rs` (278 lines)
<!-- END AUTO -->
