# AGENTS.md — convergio-server

For repo-wide rules see [../../AGENTS.md](../../AGENTS.md).

This crate is the HTTP routing shell around the core layers.

## Invariants

- Routes translate HTTP into layer calls; domain rules live in owning
  crates.
- Axum path params use `:id`, not `{id}`.
- Do not let any route bypass gates, audit, or task ownership checks.
- Keep error responses stable enough for CLI/MCP clients.
- Cross-layer E2E tests belong under this crate's `tests/`.

## Crate stats

The block below is rewritten by `cvg docs regenerate` (ADR-0015) —
do not edit between the markers.

<!-- BEGIN AUTO:crate_stats -->
**`convergio-server` stats:** 30 `*.rs` files / 33 public items / 3810 lines (under `src/`).

Files approaching the 300-line cap:
- `src/routes/graph.rs` (284 lines)
- `src/routes/fleet.rs` (277 lines)
- `src/main.rs` (255 lines)
<!-- END AUTO -->
