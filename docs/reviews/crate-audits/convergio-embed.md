# Audit — `convergio-embed`

`convergio-embed` is well bounded: storage, policy, ingest, query, and hybrid fusion are isolated and clippy-clean.
No unsafe code or production unwrap/expect paths were found.
Findings are low severity: best-effort corpus skips, stale module tables, and files close to the 300-line cap.

## Bugs / smells
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| low | crates/convergio-embed/src/corpus.rs:38 | WalkDir errors are dropped with `filter_map(Result::ok)`, hiding skipped paths from callers. | Return a small corpus report with skipped-path counts or warnings. |
| low | crates/convergio-embed/src/corpus.rs:46 | Unreadable matching files are silently skipped, so corpus coverage can shrink without signal. | Record unreadable-file counts or expose fallible collection for orchestrators. |

## Code↔doc drift
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| low | crates/convergio-embed/AGENTS.md:37 | The module layout table omits implemented `codec.rs`, `corpus.rs`, `hybrid.rs`, `ingest.rs`, `query.rs`, and `fastembed_impl.rs`. | Update the table to match the current crate surface. |
| low | crates/convergio-embed/src/lib.rs:13 | The architecture table omits public modules added for corpus, ingest, query, hybrid fusion, and feature-gated fastembed. | Expand the table or replace it with the complete public re-export list. |

## Refactor / optimization
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| low | crates/convergio-embed/src/hybrid.rs:298 | `hybrid.rs` is two lines below the 300-line cap and combines RRF, linear blend, data types, and tests. | Move linear blend or tests into a sibling module before adding more fusion modes. |
| low | crates/convergio-embed/src/fastembed_impl.rs:87 | `MultilingualE5Embedder` and `BgeM3Embedder` duplicate lazy-load and embed logic. | Extract a generic helper over model id, dimension, and `EmbeddingModel`. |

## Constitution compliance
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| low | crates/convergio-embed/src/corpus.rs:38 | Zero-tolerance error visibility is weakened by silent filesystem traversal skips. | Surface skipped-entry counts or warnings through the corpus API. |

## Confidence
high - Read `AGENTS.md`, `src/lib.rs`, every `src/**/*.rs` file, migrations, tests, and crate docs; ran `cargo clippy -p convergio-embed --all-targets -- -D warnings`.
