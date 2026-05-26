# AGENTS.md — convergio-server-core

For repo-wide rules see [../../AGENTS.md](../../AGENTS.md). For the
extraction rationale see ADR-0049 (F3 retrospective) follow-up row.

This crate is a thin seam between `convergio-server` and sibling
route crates (today only `convergio-fleet-routes`). It owns nothing
domain-shaped — just the two types every route handler needs to
agree on.

## Invariants

- **Two modules, two types.** [`state::AppState`] is the typed DI
  bag injected into every handler. [`error::ApiError`] is the
  canonical HTTP error enum with the only `IntoResponse` mapping in
  the codebase. Any other handler-shared primitive is out of scope.
- **No domain logic.** No SQL, no business rules, no audit writes.
  If a change needs `await`, it belongs in `convergio-durability`,
  `convergio-fleet`, etc. — not here.
- **No daemon dependency.** This crate must not depend on
  `convergio-server`, otherwise the cycle that motivated the split
  reopens. Server depends on us; we depend only on the domain
  layer crates whose error types `ApiError` maps.
- **All status-code mappings live in `error::IntoResponse`.** Route
  handlers do not build `Response` objects manually — they return
  `Err(ApiError::Variant { … })` and let the canonical mapping
  produce the JSON shape `{ "error": { "code", "message" } }`.
- **`From<LayerError> for ApiError` is one-way only.** We map up
  from each layer's error; we never map an `ApiError` back into a
  layer error. That direction is meaningless.

## Module layout

| File | Owns |
|------|------|
| `state.rs` | [`AppState`] struct — concrete, `Clone`, holds `Arc`s to every layer facade. |
| `error.rs` | [`ApiError`] enum, every layer-error `From` impl, and the canonical `IntoResponse`. |
| `lib.rs`   | Re-exports `AppState` and `ApiError` at the crate root. |

## Tests

No unit tests today — every variant is exercised end-to-end via
`convergio-server`'s `tests/` suite, which boots the full router
and asserts status codes + JSON bodies. If a new `ApiError` variant
gets added without an HTTP route exercising it, add the assertion
in the relevant route's `e2e_*.rs` file rather than here.

## Crate stats

The block below is rewritten by `cvg docs regenerate` (ADR-0015) —
do not edit between the markers.

<!-- BEGIN AUTO:crate_stats -->
**`convergio-server-core` stats:** 3 `*.rs` files / 4 public items / 355 lines (under `src/`).

Files approaching the 300-line cap:
- `src/error.rs` (280 lines)
<!-- END AUTO -->
