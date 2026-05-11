# Audit — `convergio-coherence`

`convergio-coherence` is broadly well split and crate-scoped clippy is clean with `-D warnings`.
The largest risk is stale documentation: the crate now ships eight subcommands, not the two/five described in local docs.
Implementation findings are concentrated in daemon-backed verifiers that silently degrade or parse the wrong response shape.

## Bugs / smells
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| medium | crates/convergio-coherence/src/plan_execution_scan.rs:108 | `fetch_bus_messages` parses `GET /v1/plans/:plan_id/messages` as NDJSON text even though the server returns a JSON array, so `bus_ok` is always false. | Decode `Vec<BusMessage>` from the JSON response and add a regression test for a non-empty bus array. |
| medium | crates/convergio-coherence/src/plan_execution_scan.rs:79 | Evidence fetch decode failures are converted to empty evidence, making strict plan-execution reports look like missing evidence instead of transport/decode failure. | Return `Result<Vec<EvidenceItem>>` and surface HTTP/decode context in the report or caller. |
| low | crates/convergio-coherence/src/close_post_hoc_scan.rs:36 | Audit page HTTP/decode failures return partial clean results, hiding verifier blind spots as zero close-post-hoc rows. | Propagate page fetch/decode errors instead of returning accumulated hits. |

## Code↔doc drift
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| low | crates/convergio-coherence/README.md:4 | The README says the suite is local-only with no daemon dependency, but `Handshake`, `PlanExecution`, and `ClosePostHoc` call the daemon. | Update the README dependency statement to list daemon-backed exceptions. |
| low | crates/convergio-coherence/README.md:6 | The README says two verifiers ship, while `CoherenceCommand` exposes eight subcommands. | Replace the stale two-item list with the current command set. |
| low | crates/convergio-coherence/AGENTS.md:11 | The crate instructions say five verifiers ship, but `CoherenceCommand` also includes `Fleet`, `ClosePostHoc`, and `PlanExecution`. | Update AGENTS.md to match the eight implemented variants. |
| low | crates/convergio-coherence/AGENTS.md:37 | The boundaries say `Handshake` is the lone HTTP verifier, but `Agents`, `ClosePostHoc`, and `PlanExecution` also use HTTP. | Restate the boundary as local-first with named daemon-backed verifiers. |
| low | crates/convergio-coherence/src/lib.rs:20 | The entry-point docs list `PlanExecution` and `Handshake` as daemon exceptions but omit `ClosePostHoc`. | Add `ClosePostHoc` to the daemon-dependent exception list. |

## Refactor / optimization
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| low | crates/convergio-coherence/src/fleet.rs:287 | `fleet.rs` is 13 lines below the 300-line cap and contains report building, rendering, and tests. | Split tests or rendering before adding more fleet checks. |
| low | crates/convergio-coherence/src/routes_parse.rs:278 | `routes_parse.rs` is 22 lines below the 300-line cap and mixes code-route parsing, docs parsing, and tests. | Move tests or document parsing helpers to a sibling module before extending route syntax support. |
| low | crates/convergio-coherence/src/plan_execution_scan.rs:67 | Daemon-backed fetch helpers duplicate soft-fallback HTTP patterns and inconsistent error handling. | Extract a small typed HTTP helper that preserves context and lets callers choose advisory degradation explicitly. |

## Constitution compliance
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| medium | crates/convergio-coherence/src/plan_execution_scan.rs:108 | No-scaffolding is weakened because the plan-execution bus check is wired but cannot observe JSON-array bus responses. | Fix response decoding and cover it with a targeted test. |
| medium | crates/convergio-coherence/src/close_post_hoc_scan.rs:36 | Zero-tolerance/tight error handling is weakened because audit scan failures can look like a clean report. | Fail closed or emit an explicit degraded finding when audit pagination cannot be trusted. |
| low | crates/convergio-coherence/src/close_post_hoc.rs:98 | Human output is hardcoded English instead of going through `convergio-i18n`. | Add EN/IT Fluent keys and render via `Bundle`. |
| low | crates/convergio-coherence/src/fleet.rs:141 | Human output is hardcoded English instead of going through `convergio-i18n`. | Add EN/IT Fluent keys and render via `Bundle`. |

## Confidence
high - Read `AGENTS.md`, `src/lib.rs`, every `src/**/*.rs` file, `README.md`, and server/i18n cross-checks; ran `cargo clippy -p convergio-coherence --all-targets -- -D warnings`.
