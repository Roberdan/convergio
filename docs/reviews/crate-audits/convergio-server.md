# Audit — `convergio-server`

`convergio-server` is broad but mostly disciplined: crate-scoped clippy is clean and the crate forbids unsafe code.
The highest-risk issues are production request paths that can panic or spawn work before task ownership is confirmed.
Docs lag the implemented HTTP surface, and several route files are close to the 300-line cap.

## Bugs / smells
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| high | crates/convergio-server/src/routes/audit/mod.rs:81 | `/v1/audit/verify` uses `expect` on a shared mutex, so a poisoned cache can panic a production request. | Convert poisoned-lock handling into an explicit 500 or recover by clearing the cache. |
| high | crates/convergio-server/src/routes/agents.rs:127 | `spawn_runner` starts the process before transitioning the task, so a failed transition leaves an unowned runner alive. | Claim or validate the task before spawning, or terminate/retire the process if the transition fails. |
| low | crates/convergio-server/src/routes/status.rs:99 | Negative `completed_limit` casts to `usize::MAX` for completed plans instead of clamping like completed tasks. | Validate or clamp `completed_limit` before both plan and task result limits. |

## Code↔doc drift
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| medium | crates/convergio-server/README.md:32 | The API surface table omits many mounted endpoints, including status, context, CRDT, workspace, graph, embed, fleet, telemetry, actions, and gates. | Mark the table as a quickstart subset or regenerate it from `app::router`/actions metadata. |
| low | crates/convergio-server/src/lib.rs:4 | The crate docs say the router holds Layer 1 `Durability`, but `AppState` also wires bus, lifecycle, graph, embed, and fleet facades. | Reword the entry doc to describe the full routing shell state. |
| low | crates/convergio-server/README.md:51 | The API table documents `/v1/agents/spawn` but omits the implemented `/v1/agents/spawn-runner` route. | Add the runner endpoint or point readers to generated action discovery for the full surface. |

## Refactor / optimization
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| low | crates/convergio-server/src/routes/graph.rs:1 | `graph.rs` is 295 lines and one feature away from the 300-line cap. | Split hybrid semantic assembly or graph query DTOs into a sibling module. |
| low | crates/convergio-fleet-routes/src/fleet.rs:1 | `fleet.rs` is 277 lines and mixes repo CRUD, build orchestration, and pattern queries. | Split fleet repo CRUD from build/pattern route handlers. |
| low | crates/convergio-server/src/error.rs:1 | `error.rs` is 266 lines and centralizes unrelated durability, bus, lifecycle, graph, and fleet mappings. | Split domain-specific error mapping helpers while preserving the stable JSON envelope. |
| low | crates/convergio-server/src/routes/messages.rs:57 | Message-limit validation is duplicated across plan, system, and context message routes. | Extract a small shared limit validator with route-specific error codes. |

## Constitution compliance
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| high | crates/convergio-server/src/routes/agents.rs:127 | P2/P4 risk: runner process creation can precede the audited task ownership transition. | Make runner spawning ownership-first and audit-visible before local process execution. |
| medium | crates/convergio-server/src/error.rs:111 | P5 risk: HTTP error messages are hardcoded English and are returned to CLI/MCP clients. | Return stable codes plus localized messages or move user-facing text through the i18n catalog. |
| low | crates/convergio-server/src/lib.rs:9 | P2 positive: the crate forbids unsafe code and no unsafe blocks were found. | Keep `#![forbid(unsafe_code)]` in place. |

## Confidence
high — Read `AGENTS.md`, `README.md`, `src/lib.rs`, walked all 36 `src/**/*.rs` files, grepped panic/unsafe/debt markers, checked line counts, cross-checked docs against mounted routes, and ran `cargo clippy -p convergio-server --all-targets -- -D warnings` cleanly.
