# Audit — `convergio-cli`

`convergio-cli` is broadly healthy and crate-scoped clippy is clean with `-D warnings`.
The strongest finding is operational drift: the generated launchd plist contradicts the post-incident daemon restart guard rails.
Most remaining issues are low/medium maintainability or contract drift around ignored flags, shared internal types, and swallowed cleanup errors.

## Bugs / smells
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| medium | crates/convergio-cli/src/commands/service.rs:185 | The launchd plist sets `RunAtLoad` to true, recreating the auto-start behavior the incident playbook says must stay disabled. | Generate `RunAtLoad=false` unless an explicit operator flag opts in. |
| medium | crates/convergio-cli/src/commands/service.rs:186 | The launchd plist sets `KeepAlive` to true, so a crashing daemon can be respawned repeatedly. | Generate `KeepAlive=false` to match the documented guard rail. |
| medium | crates/convergio-cli/src/commands/fleet_cleanup.rs:70 | Cleanup ignores `git worktree remove` failures while still reporting the worktree as removed. | Propagate the error or record per-item failures in the report. |
| medium | crates/convergio-cli/src/commands/agent_spawn_wire.rs:49 | Unknown task statuses are silently coerced to `pending` when parsing spawn input. | Return a parsing error for unknown daemon statuses instead of defaulting. |

## Code↔doc drift
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| medium | AGENTS.md:506 | Root guidance says launchd should set `KeepAlive=false` and `RunAtLoad=false`, but `service.rs` emits both as true. | Align `cvg service install` with the documented post-incident service policy. |
| low | crates/convergio-cli/README.md:19 | The README says the CLI does not import internal server crates, but `Cargo.toml` depends on `convergio-durability`. | Replace durability model imports with API DTOs or document the allowed shared-type exception. |
| low | crates/convergio-cli/src/commands/setup.rs:32 | `cvg setup fleet --force` is documented as re-registering repos, but `setup_fleet.rs` ignores the flag. | Implement force semantics or remove the flag claim. |

## Refactor / optimization
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| low | crates/convergio-cli/src/commands/graph.rs:300 | `graph.rs` sits exactly at the 300-line cap and will fail the next added line. | Split rendering or request-building helpers into a sibling module. |
| low | crates/convergio-cli/src/commands/task.rs:299 | `task.rs` mixes create/list/render-transition orchestration and has one line of headroom. | Move transition or completion helpers into focused modules. |
| low | crates/convergio-cli/src/commands/discover.rs:297 | `discover.rs` is within three lines of the cap while combining peer, bus, and plan snapshot logic. | Split discovery rendering or fetch phases before extending it. |
| low | crates/convergio-cli/src/commands/docs_generators.rs:156 | The ADR index generator leaves a dead `let _ = file` placeholder for skipped glob logic. | Remove the placeholder or implement the intended directory scan. |

## Constitution compliance
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| medium | crates/convergio-cli/src/commands/service.rs:185 | Zero-tolerance operational safety is weakened by generating a service that starts automatically after the documented incident pattern. | Default launchd `RunAtLoad` to false. |
| medium | crates/convergio-cli/src/commands/service.rs:186 | Zero-tolerance operational safety is weakened by generating a service that respawns after crashes. | Default launchd `KeepAlive` to false. |
| low | crates/convergio-cli/src/commands/setup_fleet.rs:18 | No-scaffolding is weakened because the public `--force` path is accepted but unused. | Wire the flag through to behavior or remove it. |

## Confidence
high — Read `AGENTS.md`, attempted `src/lib.rs` and found this is a binary crate, read `src/main.rs`, `README.md`, and all `src/**/*.rs`; ran `cargo clippy -p convergio-cli --all-targets -- -D warnings`.
