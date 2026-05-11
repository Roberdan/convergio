# Audit — `convergio-lifecycle`

`convergio-lifecycle` has no unsafe code or production `unwrap`/`expect` paths found.
Crate-scoped clippy is clean with `-D warnings`.
Main risks are swallowed process I/O errors, silent status coercion, and small doc/API drift.

## Bugs / smells
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| medium | crates/convergio-lifecycle/src/supervisor.rs:132 | `stdin_payload` write failures are ignored, so a spawned runner can miss its prompt while the API reports success. | Propagate the write error, kill the child, and mark the process row failed. |
| medium | crates/convergio-lifecycle/src/supervisor.rs:277 | Unknown persisted statuses are silently coerced to `Failed`, hiding database invariant drift. | Return a typed invalid-status error instead of mapping unknown values to `Failed`. |
| low | crates/convergio-lifecycle/src/stdout_relay.rs:19 | `lines.next_line()` read errors end the relay loop without a warning. | Log the read error before terminating the relay task. |

## Code↔doc drift
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| low | crates/convergio-lifecycle/README.md:21 | The `SpawnSpec` API example omits implemented `cwd` and `stdin_payload` fields. | Update the API table to show the full current `SpawnSpec` surface. |
| low | crates/convergio-lifecycle/README.md:38 | The README says missing heartbeats are noticed in 60s, but the lifecycle crate only records heartbeats and watches PIDs. | Rephrase heartbeat recovery as Layer 1 reaper behavior and avoid a lifecycle-owned 60s claim. |

## Refactor / optimization
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| low | crates/convergio-lifecycle/src/supervisor.rs:60 | `spawn_with_timeout` handles row creation, command setup, stdin, stdout relay, timeout cleanup, and response mapping in one near-cap function. | Extract child command setup and post-spawn bookkeeping helpers before adding more runner behavior. |
| low | crates/convergio-lifecycle/src/supervisor_list.rs:37 | `ProcessListRow` duplicates the `ProcessRow` conversion and timestamp parsing in `supervisor.rs`. | Share a row conversion helper or reuse the crate-level timestamp parsers. |

## Constitution compliance
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| medium | crates/convergio-lifecycle/src/supervisor.rs:132 | Zero-tolerance reliability is weakened by silently accepting failed runner prompt delivery. | Treat prompt delivery failure as a spawn failure with explicit error context. |
| low | crates/convergio-lifecycle/README.md:38 | Zero-tolerance doc accuracy is weakened by attributing heartbeat recovery timing to this crate. | Update the README to distinguish lifecycle heartbeat storage from durability reaping. |

## Confidence
high — Read `AGENTS.md`, `README.md`, `src/lib.rs`, every `src/**/*.rs` file, migrations, and ran `cargo clippy -p convergio-lifecycle --all-targets -- -D warnings`.
