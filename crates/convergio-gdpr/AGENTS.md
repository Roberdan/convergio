# AGENTS.md — `convergio-gdpr`

## Responsibility

Leaf GDPR data-subject-rights handlers for Article 15 access,
Article 17 erasure tombstones, and Article 20 portability exports.
Callers supply subject-scoped records; this crate has no DB or HTTP.

## Boundaries

- No direct dependency on `convergio-server` or `convergio-durability`.
- Keep `lib.rs` ≤ 300 lines.
- Every public item carries a `///` doc; crate-level `//!` is mandatory.

## Invariants

- Do not put PII in error messages; errors stay structural.
- Unsupported rights return `GdprError::UnsupportedRight` until their
  storage semantics are designed.
- Response payloads must be structured JSON, not stringly summaries.

## Tests

```bash
cargo test -p convergio-gdpr
cargo clippy -p convergio-gdpr --all-targets -- -D warnings
```
