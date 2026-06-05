# AGENTS. convergio-connectormd 

For repo-wide rules see [../../AGENTS.md](../../AGENTS.md).

This crate owns the **Connector SDK core** (ADR-0057):
- the `Connector` trait (discover/pull/watermark/schema_hash/health)
- YAML crosswalk mapping parsing + stable schema hashing
- sandboxed connector runner (separate process, scoped creds, rate limit, backoff)
- contract-test helpers for connector implementations

## Invariants

- No network credentials are loaded implicitly from the environment. The
  runner only passes explicitly provided keys/values.
- Rate-limit and backoff are enforced in the runner for retryable
  connector failures.
- Public API is small and typed; YAML is validated at the boundary.
- No `unwrap()`/`expect()` in `src/` (tests exempt).

## Tests

- Unit tests for YAML parsing + stable hashing.
- Integration tests for the sandboxed runner using a tempdir shim script.
