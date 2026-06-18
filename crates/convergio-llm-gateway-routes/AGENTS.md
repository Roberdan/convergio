# AGENTS.md — convergio-llm-gateway-routes

For repo-wide rules see [../../AGENTS.md](../../AGENTS.md).

LLM Gateway HTTP routes extracted from `convergio-server` (same pattern
as `convergio-fleet-routes` and `convergio-ontology-routes`). Exists to
keep the daemon crate under its context-budget cap and to host the
internal LLM gateway surface (`POST /v1/llm-gateway/call`).

## Invariants

- Routes translate HTTP into provider/cache calls; the gateway logic
  (routing, allow-lists, token caps, egress redaction, schema fence,
  provenance) lives here, not in `convergio-server`.
- Axum path params use `:id`, not `{id}`.
- Share the canonical `convergio_server_core::AppState` and return
  `convergio_server_core::ApiError`. No new `IntoResponse` mapping here.
- Behaviour must stay byte-identical to the pre-extraction routes; the
  cross-layer E2E tests (`e2e_llm_gateway`) live in
  `convergio-server/tests/`.
- Do not depend on `convergio-server` (that reopens the cycle the split
  closed).

## Crate stats

The block below is rewritten by `cvg docs regenerate` (ADR-0015) —
do not edit between the markers.

<!-- BEGIN AUTO:crate_stats -->
**`convergio-llm-gateway-routes` stats:** 7 `*.rs` files / 1 public items / 1072 lines (under `src/`).

Files approaching the 300-line cap:
- `src/lib.rs` (299 lines)
- `src/redact.rs` (253 lines)
<!-- END AUTO -->
