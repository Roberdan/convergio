# Audit — `convergio-mcp`

Clippy is clean for `convergio-mcp`.
The bridge mostly respects its constrained two-tool, daemon-backed boundary.
The main issue is that `explain_last_refusal` advertises a task filter but ignores the caller's params.

## Bugs / smells
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| medium | src/actions.rs:54 | `Action::ExplainLastRefusal` discards `request.params`, so a requested `task_id` filter can return an unrelated latest refusal. | Pass params into `explain_last_refusal` and prefer the supplied `task_id` over bridge-local memory. |
| low | src/bridge.rs:83 | MCP action logging silently returns when `HOME` is unavailable, losing diagnostics without any trace event. | Emit a non-secret `tracing::warn!` when the log path cannot be resolved. |
| low | src/http.rs:38 | Invalid daemon JSON is collapsed to `{}`, so a malformed daemon response can look like a successful empty payload. | Return an explicit protocol-mapping error when response JSON decoding fails. |

## Code↔doc drift
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| medium | src/help.rs:270 | Help documents `explain_last_refusal` as accepting `task_id`, but dispatch ignores that parameter. | Wire the documented `task_id` parameter through dispatch and add a regression test. |

## Refactor / optimization
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| low | src/actions.rs:1 | `actions.rs` is 287 lines and already close to the repo's 300-line Rust cap. | Split action-specific path/body builders into focused modules before adding more actions. |
| low | src/help.rs:75 | The 198-line `action_help` match couples all action schemas in one near-cap file. | Move action help snippets into capability-focused helpers generated or grouped by action family. |
| low | src/bridge.rs:82 | `bridge.rs` mixes tool declarations, file logging, and integration-test daemon bootstrapping in a 254-line file. | Extract logging helpers or test support into sibling modules before adding behavior. |

## Constitution compliance
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| medium | src/actions.rs:54 | Zero-tolerance reliability is weakened because refusal explanation can report the wrong task's gate failure. | Honor the caller's `task_id` filter so recovery guidance stays task-scoped. |
| low | src/bridge.rs:83 | Zero-tolerance observability is weakened by silently dropping MCP diagnostics when log setup fails. | Log non-sensitive setup failures through `tracing` while keeping payload data out of diagnostics. |

## Confidence
high — Read `AGENTS.md`, `README.md`, entry-point `src/main.rs`, every `src/**/*.rs` file, and ran crate-scoped clippy cleanly.
