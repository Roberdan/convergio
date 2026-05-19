# AGENTS.md — convergio-fleet-routes

For repo-wide rules see [../../AGENTS.md](../../AGENTS.md). For the
extraction rationale see ADR-0049 (F3 retrospective) — the F3 phase
flagged this crate as the next structural cleanup after the
`convergio-server` 14000-line hard cap hit during F3-5.

This crate owns the HTTP route handlers for the fleet surface
(ADR-0038, F2 + F3). It is intentionally a thin axum layer:

| Concern | Owner |
|---------|-------|
| Route handlers + query parsing | this crate |
| Domain logic | `convergio-fleet`, `convergio-embed`, `convergio-graph` |
| `AppState` + `ApiError` | `convergio-server-core` |
| Top-level router assembly + middleware | `convergio-server` |

## Invariants

- **No daemon dependency.** This crate does not depend on
  `convergio-server` — that would close the cycle. Server depends on
  it.
- **State + error live in `convergio-server-core`.** Do not redefine
  `AppState` or `ApiError` here; importing them is the contract.
- **Behaviour stays byte-identical** to the pre-extraction routes.
  Any change to error shape or status code is a separate PR.
- **Per-file 300-line cap applies** (CONSTITUTION + lefthook G2).

## Tests

Cross-layer E2E lives in `crates/convergio-server/tests/` and boots
the full router (which mounts this crate). Per-route unit tests can
live alongside each module via `#[cfg(test)] mod tests` for pure
helpers (e.g. `parse_repo_pair` in `fleet_duplicates.rs`).
