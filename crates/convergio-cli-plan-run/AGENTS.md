# AGENTS.md — convergio-cli-plan-run

For repo-wide rules see [../../AGENTS.md](../../AGENTS.md).

This crate hosts the `cvg plan run` orchestrator extracted from
`convergio-cli` to honour the per-crate hard cap. Mirrors the
ADR-0041 split for `cvg session` and ADR-0040 for `cvg pr`.

## Invariants

- No back-edge on `convergio-cli`: this crate cannot import from it.
  The CLI shim adapts its own `Client` / `OutputMode` to the local
  types defined in `lib.rs`.
- Stay an HTTP client; do not import server crates or write SQLite
  directly.
- Respect daemon gates and audit (no bypass).
- Keep files under the 300-line Rust cap.
