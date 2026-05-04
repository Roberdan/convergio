# AGENTS.md — convergio-cli-pr

For repo-wide rules see [../../AGENTS.md](../../AGENTS.md).

This crate hosts the `cvg pr` suite extracted from `convergio-cli` to honour the per-crate hard cap. Mirrors the ADR-0041 split for `cvg session`.

## Invariants

- No back-edge on `convergio-cli`: this crate cannot import from it. The CLI shim adapts its own `Client` / `OutputMode` to the local types.
- Stay an HTTP client; do not import server crates or write SQLite directly.
- Respect daemon gates and audit (no bypass).
- Keep files under the 300-line Rust cap.
