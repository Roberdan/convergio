# Audit — `convergio-fleet`

`convergio-fleet` is mostly cohesive: clippy is clean, unsafe is forbidden, and production code avoids `unwrap()` / `expect()`.
The main code issue is that the public similarity upsert can persist sub-threshold edges despite its threshold-oriented contract.
The main documentation issue is stale crate guidance that still marks cross-repo similarity edges as future work.

## Bugs / smells
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| medium | src/similar.rs:83 | `upsert_similar_edge` stores any score below `DUPLICATES_THRESHOLD` as `similar_to`, including scores below the documented 0.85 threshold. | Reject or no-op scores below `SIMILAR_TO_THRESHOLD`, and add a regression test. |

## Code↔doc drift
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| low | crates/convergio-fleet/AGENTS.md:62 | The F2 checklist says cross-repo similarity edges are later work, but `similar`, `batch`, `patterns`, `duplicates`, migrations 0801/0802, and cross-language tests are implemented. | Mark the similarity-edge deliverable complete and list the implemented modules. |
| low | crates/convergio-fleet/AGENTS.md:39 | The module layout table omits implemented public modules `batch`, `similar`, `patterns`, `duplicates`, and `recall`. | Refresh the table so local guidance matches the actual crate surface. |

## Refactor / optimization
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| low | src/batch.rs:139 | `batch.rs` is near the 300-line cap because unit tests live inline with the batch implementation. | Move tests to `src/batch_tests.rs` like `patterns` and `duplicates`. |
| low | src/similar.rs:173 | `similar.rs` is near the 300-line cap because store extension methods and tests share one file. | Move tests to `src/similar_tests.rs` or split mapping helpers from store methods. |

## Constitution compliance
_None._

## Confidence
high — Read `AGENTS.md`, `src/lib.rs`, every `src/**/*.rs`, migrations, integration tests, and ran `cargo clippy -p convergio-fleet --all-targets -- -D warnings`.
