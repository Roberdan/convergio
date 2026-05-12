# AGENTS.md — convergio-embed

For repo-wide rules see [../../AGENTS.md](../../AGENTS.md). For the
decision behind this crate see
[../../docs/adr/0038-fleet-retrieval-cross-repo-graph.md](../../docs/adr/0038-fleet-retrieval-cross-repo-graph.md).
For methodology behind the recall gate see
[../../docs/spec/fleet-retrieval-golden-methodology.md](../../docs/spec/fleet-retrieval-golden-methodology.md).

This crate owns embeddings persistence + the [`Embedder`] trait. It
does **not** bundle a model by default — the `fastembed` feature adds
real `fastembed-rs`-backed embedders that download into
`~/.convergio/v3/models/` on first use.

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
- **Pure-Rust brute-force KNN (for now).** The `graph_vec_index` vec0
  virtual table is created by migration 0700, but queries still use
  brute-force cosine today; F1-β swaps the KNN implementation to
  `sqlite-vec` while keeping [`EmbedStore`]’s signature stable.

## Module layout

| File | Owns |
|------|------|
| `codec.rs`          | Internal `Vec<f32>` ⇄ blob round-trip (little-endian) |
| `corpus.rs`         | [`collect_files`] / [`collect_files_report`] — filesystem walk → [`IngestNode`] inputs (+ skip counters) |
| `embedder.rs`       | [`Embedder`] trait + `testing::DeterministicTestEmbedder` |
| `error.rs`          | [`EmbedError`] |
| `fastembed_impl.rs` | Feature-gated `fastembed-rs` embedders (`MultilingualE5Embedder`, `BgeM3Embedder`) |
| `hybrid.rs`         | RRF + linear-blend fusion for structural ⊕ semantic retrieval |
| `ingest.rs`         | Batch embed pipeline: [`ingest`] / [`ingest_one`] + [`IngestReport`] |
| `migrate.rs`        | Migration runner (range 700-799, ADR-0003) |
| `query.rs`          | [`semantic_search`] — semantic-only KNN over the store |
| `select.rs`         | [`EmbedPolicy`] — which targets get embedded |
| `source.rs`         | [`SourceText`] — canonical text + SHA-256 hash |
| `store.rs`          | [`EmbedStore`] — persistence + brute-force cosine KNN |

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
**`convergio-embed` stats:** 14 `*.rs` files / 47 public items / 1985 lines (under `src/`).

Files approaching the 300-line cap:
- `src/corpus.rs` (295 lines)
- `src/store.rs` (265 lines)
<!-- END AUTO -->
