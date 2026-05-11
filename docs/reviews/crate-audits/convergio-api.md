# Audit — `convergio-api`

`convergio-api` is a compact schema crate with no runtime IO, unsafe code, or production unwrap/expect paths found.
Crate-scoped clippy is clean with `-D warnings`; the generated registry matches the closed `Action` enum.
The only actionable items are a doc/invariant mismatch around build-time IO and a near-cap source file.

## Bugs / smells
_None._

## Code↔doc drift
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| low | crates/convergio-api/AGENTS.md:6 | The crate invariant says it must not perform IO, but `build.rs` reads `src/lib.rs` and writes `actions.json` during builds. | Clarify that the prohibition is runtime IO, or move generated-file writes out of the package source tree. |

## Refactor / optimization
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| low | crates/convergio-api/src/action.rs:295 | `Action` is five lines below the 300-line cap and combines names, capabilities, summaries, and compensation metadata in one file. | Split metadata tables or compensation mapping into a small sibling module before adding more actions. |

## Constitution compliance
_None._

## Confidence
high - Read `AGENTS.md`, `src/lib.rs`, every `src/**/*.rs` file, `README.md`, and `build.rs`; ran `cargo clippy -p convergio-api --all-targets -- -D warnings`.
