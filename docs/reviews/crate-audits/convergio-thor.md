# Audit — `convergio-thor`

Small validator crate with clean clippy and no production panic/unsafe paths found.
Main behavior is simple and covered by focused integration tests, including pipeline timeout/truncation paths.
The main issue is entry-point documentation drift around `submitted` tasks being promoted to `done`.

## Bugs / smells
_None._

## Code↔doc drift
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| medium | crates/convergio-thor/src/lib.rs:10 | The crate-level MVP rule says `Pass` requires every task to already be `done`, but `validate` accepts `submitted` tasks and promotes them. | Update the entry-point docs to match the submitted-or-done validator behavior. |

## Refactor / optimization
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| low | crates/convergio-thor/src/thor.rs:180 | `task.id.clone()` is avoidable because the owned task is not used after promotion collection. | Move `task.id` into `to_promote` after evidence checks. |

## Constitution compliance
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| low | crates/convergio-thor/src/thor.rs:137 | Validator failure reasons are hardcoded English strings that can surface through API/CLI responses. | Route operator-facing verdict text through the i18n layer or return stable reason codes plus localized rendering. |

## Confidence
high — Read `AGENTS.md`, `README.md`, `src/lib.rs`, every `src/**/*.rs` file, integration tests, cross-checked durability evidence/promotion paths, and ran `cargo clippy -p convergio-thor --all-targets -- -D warnings`.
