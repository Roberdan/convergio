# AGENTS.md — convergio-ontology-routes

For repo-wide rules see [../../AGENTS.md](../../AGENTS.md).

Ontology Runtime HTTP routes extracted from `convergio-server`
(ADR-0049 follow-up; same pattern as `convergio-fleet-routes`). Exists
to keep the daemon crate under its context-budget cap and to host future
ontology/purpose route additions.

## Invariants

- Routes translate HTTP into layer calls; domain rules live in
  `convergio-ontology` / `convergio-durability`, never here.
- Axum path params use `:id`, not `{id}`.
- Share the canonical `convergio_server_core::AppState` and return
  `convergio_server_core::ApiError`. No new `IntoResponse` mapping here.
- Behaviour must stay byte-identical to the pre-extraction routes; the
  cross-layer E2E tests live in `convergio-server/tests/`.
- Do not depend on `convergio-server` (that reopens the cycle the split
  closed).

## Crate stats

The block below is rewritten by `cvg docs regenerate` (ADR-0015) —
do not edit between the markers.

<!-- BEGIN AUTO:crate_stats -->
**`convergio-ontology-routes` stats:** 6 `*.rs` files / 6 public items / 770 lines (under `src/`).

Files approaching the 300-line cap:
- `src/ontology.rs` (262 lines)
<!-- END AUTO -->
