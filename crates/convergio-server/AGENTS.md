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
**`convergio-server` stats:** 24 `*.rs` files / 23 public items / 2390 lines (under `src/`).

No files within 50 lines of the 300-line cap.
<!-- END AUTO -->
