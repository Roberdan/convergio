# AGENTS.md — convergio-embed

For repo-wide rules see [../../AGENTS.md](../../AGENTS.md). For the
decision behind this crate see
[../../docs/adr/0038-fleet-retrieval-cross-repo-graph.md](../../docs/adr/0038-fleet-retrieval-cross-repo-graph.md).
For methodology behind the recall gate see
[../../docs/spec/fleet-retrieval-golden-methodology.md](../../docs/spec/fleet-retrieval-golden-methodology.md).

This crate owns embeddings persistence + the [`Embedder`] trait. It
does **not** bundle a model — F1-α ships only a deterministic test
embedder. The real `fastembed-rs`-backed model arrives in F1-β.

## Invariants

- **Embedder is a trait, never a concrete type leaked into APIs.**
  Storage and policy do not assume which model produced a vector;
  rows are keyed by `(repo, node_id, model)` so multiple models can
  coexist.
- **Determinism** — every implementation must produce the same vector
  for the same input on the same hardware family. CI assumes this
  (golden-set methodology § 6).
- **No network at retrieval time.** Models that download weights do
  it at *first use* into `~/.convergio/v3/models/`, never inside an
  `EmbedStore` query path.
- **Re-embed trigger is `source_hash`, not mtime.** Formatter touches
  must not invalidate cached embeddings.
- **Selective embedding.** [`EmbedPolicy::should_embed`] is the
  single source of truth for which node categories get embedded;
  callers must not bypass it.
- **Migration range 700-799** reserved by ADR-0003.
- **Pure-Rust brute-force KNN in F1-α.** `sqlite-vec` virtual table +
  extension load arrive in F1-β; the [`EmbedStore`] signature stays
  stable across the swap so callers do not change.

## Module layout

| File | Owns |
|------|------|
| `embedder.rs` | [`Embedder`] trait + `testing::DeterministicTestEmbedder` |
| `source.rs`   | [`SourceText`] — canonical text + SHA-256 hash |
| `select.rs`   | [`EmbedPolicy`] — which targets get embedded |
| `store.rs`    | [`EmbedStore`] — persistence + brute-force cosine KNN |
| `migrate.rs`  | Migration runner (range 700-799, ADR-0003) |
| `error.rs`    | [`EmbedError`] |

## Tests

- Per-module `#[cfg(test)] mod tests` covers pure functions
  (cosine, blob roundtrip, hash stability, policy decisions).
- `tests/store_e2e.rs` exercises the full store API against a
  tempdir SQLite via `convergio-db::Pool`. Real Pool, no mocks.
- Cross-layer E2E lives in `crates/convergio-server/tests/e2e_embed.rs`.

## Crate stats

The block below is rewritten by `cvg docs regenerate` (ADR-0015) —
do not edit between the markers.

<!-- BEGIN AUTO:crate_stats -->
**`convergio-embed` stats:** 13 `*.rs` files / 40 public items / 1617 lines (under `src/`).

Files approaching the 300-line cap:
- `src/store.rs` (264 lines)
<!-- END AUTO -->
