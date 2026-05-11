# Audit — `convergio-durability`

`convergio-durability` is broadly well split and clippy-clean for the crate-scoped all-targets check.
The main risks are silent coercion of corrupted persisted values and swallowed store errors in agent registration/projections.
Docs mostly match implementation, with two gate/audit-invariant drift points to fix.

## Bugs / smells
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| medium | crates/convergio-durability/src/store/tasks.rs:234 | Invalid persisted task statuses are silently treated as `Pending`. | Return a parse error so corrupted task rows cannot re-enter scheduling as pending work. |
| medium | crates/convergio-durability/src/store/tasks.rs:236 | Invalid `evidence_required` JSON is silently treated as an empty requirement set. | Propagate JSON parse failures with row context instead of defaulting to `[]`. |
| medium | crates/convergio-durability/src/store/agents.rs:91 | `register()` turns every `get()` failure into "agent absent" via `.ok()`. | Only ignore `NotFound`; propagate database and decode errors. |
| medium | crates/convergio-durability/src/agent_facade.rs:26 | `register_agent()` swallows prior-agent lookup failures before deciding whether to append `agent.session_started`. | Only treat `NotFound` as no prior agent so telemetry audit rows are not emitted from failed reads. |
| medium | crates/convergio-durability/src/store/agent_queries.rs:55 | Invalid persisted agent timestamps fall back to `Utc::now()`. | Return a projection error or omit the bad row with an explicit diagnostic instead of making stale/corrupt rows look fresh. |
| medium | crates/convergio-durability/src/store/agent_claims.rs:13 | Invalid claimed-task timestamps fall back to `Utc::now()`. | Propagate timestamp parse failures so corrupt task ownership projections are visible. |

## Code↔doc drift
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| medium | crates/convergio-durability/AGENTS.md:58 | The invariant says every `Durability` state change writes exactly one audit row, but `register_agent()` can append both `agent.registered` and `agent.session_started`. | Clarify the invariant or split session telemetry so the documented audit cardinality is true. |
| low | crates/convergio-durability/src/lib.rs:21 | The module map mentions an `identity` gate, but `default_pipeline()` contains no identity gate. | Update the doc comment to list the actual gates or reintroduce a named identity gate if intended. |

## Refactor / optimization
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| low | crates/convergio-durability/src/audit/action.rs:1 | `audit/action.rs` is 296 lines and combines action parsing, compensation modeling, and inverse construction. | Split compensation/inverse construction into a sibling module before the next action expansion. |
| low | crates/convergio-durability/src/facade_transitions.rs:1 | `facade_transitions.rs` is 294 lines and mixes gate execution, audit payload construction, and duration calculations. | Extract audit payload/status-duration helpers to keep transition orchestration below the cap. |

## Constitution compliance
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| medium | crates/convergio-durability/src/store/tasks.rs:234 | P1 zero-tolerance is weakened because corrupt task state is silently normalized instead of refused. | Treat invalid persisted state as an explicit durability error. |

## Confidence
high - Read crate instructions and entry docs, walked `src/**/*.rs` with targeted risk scans plus hotspot reads, cross-checked docs, and ran `cargo clippy -p convergio-durability --all-targets -- -D warnings`.
