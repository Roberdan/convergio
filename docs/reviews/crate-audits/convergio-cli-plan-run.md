# Audit — `convergio-cli-plan-run`

Small focused crate with clean clippy output and no panic/unsafe usage in non-test paths.
The main risk is cancellation behavior around concurrent task transitions.
Docs match the implemented CLI shim and HTTP-client-only boundary.

## Bugs / smells
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| medium | src/runner.rs:183 | Breaking on the first failed future drops other in-flight submit futures and can leave already-claimed tasks stuck `in_progress`. | Stop scheduling new tasks after the first failure, but drain existing in-flight submissions before returning. |
| low | src/runner.rs:230 | The plan-scoped bus publish error is swallowed, hiding coordination-message failures from callers and audits. | Propagate the error or log/report an explicit non-fatal warning consistent with CLI output modes. |

## Code↔doc drift
_None._

## Refactor / optimization
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| low | src/runner.rs:1 | `runner.rs` is 295 lines, leaving only five lines before the repo's 300-line Rust cap. | Split HTTP transition/publish helpers or tests into a small sibling module before adding behavior. |

## Constitution compliance
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| medium | src/runner.rs:183 | Zero-tolerance reliability is weakened because cancellation can strand durable tasks in an intermediate state. | Drain in-flight transitions and make failure handling leave daemon state deterministic. |
| low | src/runner.rs:230 | Zero-tolerance observability is weakened by silently discarding bus publish failures. | Surface the publish failure as an error or explicit localized warning. |

## Confidence
high — Read AGENTS.md, README.md, src/lib.rs, all src/**/*.rs, the CLI shim, i18n strings, and ran crate-scoped clippy cleanly.
