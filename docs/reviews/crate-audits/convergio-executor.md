# Audit — `convergio-executor`

The crate is focused and crate-scoped clippy is clean.
The main risk is duplicate dispatch because task selection and promotion are not an atomic claim.
Docs lag the implemented runner/worktree path, and `executor.rs` is still one edit away from the 300-line cap.

## Bugs / smells
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| high | crates/convergio-executor/src/executor.rs:89 | Dispatch reads pending tasks before spawning and only transitions after spawn, so concurrent ticks can spawn the same task twice. | Add an audited atomic claim or conditional pending-to-in_progress transition before any runner spawn. |
| low | crates/convergio-executor/src/worktree.rs:76 | Non-UTF-8 worktree paths are converted to an empty string for `git worktree add`. | Pass `Path`/`OsStr` arguments to `Command` instead of lossy string slices. |

## Code↔doc drift
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| medium | crates/convergio-executor/src/lib.rs:14 | The crate docs describe only the legacy `SpawnTemplate` MVP path, but the implementation defaults to runner/worktree dispatch when task runner fields or `CONVERGIO_EXECUTOR_USE_RUNNER` are present. | Update the entry-point docs to describe both legacy and runner-based dispatch paths. |
| low | crates/convergio-executor/src/lib.rs:20 | The docs say the task id is the only spawned arg, but `SpawnTemplate::default()` prepends `"task"`. | Reword the legacy path docs to say the task id is appended to template args. |
| low | crates/convergio-executor/src/heartbeat.rs:4 | The module docs say agents do not call `cvg agent heartbeat`, but the current CLI has no such subcommand. | Update the doc to name the actual heartbeat API or restore the documented CLI command. |

## Refactor / optimization
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| low | crates/convergio-executor/src/executor.rs:1 | `executor.rs` is 294 lines and already near the repo's 300-line cap. | Split SQL discovery/counting or runner-spawn assembly into a focused helper module. |

## Constitution compliance
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| high | crates/convergio-executor/src/executor.rs:108 | Duplicate-dispatch race violates the claim-state invariant in `AGENTS.md` and can create competing agents for one task. | Make dispatch ownership atomic and audit-visible before process creation. |
| medium | crates/convergio-executor/src/executor.rs:222 | Worktree refusal text is hardcoded English that can surface through daemon/API errors. | Route operator-facing refusal messages through the i18n message catalog or return stable reason codes. |

## Confidence
high — Read `AGENTS.md`, `README.md`, every `src/**/*.rs` file, grepped for panic/unsafe/debt markers, checked file line counts, cross-checked doc claims against code, and ran crate-scoped clippy cleanly.
