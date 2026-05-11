# Audit — `convergio-bus`

`convergio-bus` has no unsafe code or production unwrap/expect paths found.
Crate-scoped clippy is clean with `-D warnings`.
Main findings are stale README claims plus unchecked caller limits on read queries.

## Bugs / smells
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| medium | crates/convergio-bus/src/bus.rs:89 | `poll` accepts caller-provided `limit` directly, so `LIMIT -1` can become an unbounded read. | Reject non-positive limits and cap maximum page size before binding. |
| medium | crates/convergio-bus/src/bus_inspection.rs:59 | `tail` accepts caller-provided `limit` directly, so inspection reads can become unbounded. | Validate and cap `limit` consistently with `poll`. |
| medium | crates/convergio-bus/src/bus_system.rs:67 | `poll_system` accepts caller-provided `limit` directly, so system-topic reads can become unbounded. | Validate and cap `limit` consistently with plan-scoped reads. |

## Code↔doc drift
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| low | crates/convergio-bus/README.md:14 | The API table omits implemented `publish_system`, `poll_system`, `poll_filtered`, `tail`, and `topics` surfaces. | Update the README API table to match the current public `Bus` methods. |
| low | crates/convergio-bus/README.md:34 | The README says the bus is not cross-plan or system-wide broadcast, but `system.*` topics with `plan_id IS NULL` are implemented. | Document the ADR-0025 `system.*` exception instead of saying it is absent. |
| low | crates/convergio-bus/AGENTS.md:10 | The invariant says messages are scoped to a plan, but system-scoped messages intentionally have no plan. | Add the same narrow `system.*` exception described in `src/lib.rs`. |

## Refactor / optimization
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| low | crates/convergio-bus/src/bus.rs:124 | The `agent_messages` select list is duplicated across plan, system, and inspection queries. | Extract a small query helper or constant projection before adding more read paths. |
| low | crates/convergio-bus/migrations/0103_system_topics.sql:55 | `idx_agent_messages_system_topic` is ordered by `created_at`, while `poll_system` filters on `seq > ?` and orders by `seq`. | Add a future migration with an index shaped for `(topic, consumed_at, seq)` or `(topic, seq)` for system polling. |

## Constitution compliance
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| medium | crates/convergio-bus/src/bus.rs:89 | Security-first is weakened because public read APIs trust caller limits and can perform unbounded reads. | Enforce positive bounded limits at the crate boundary. |
| low | crates/convergio-bus/README.md:34 | Zero-tolerance doc accuracy is violated by stale system-broadcast documentation. | Update the README to match implemented `system.*` behavior. |

## Confidence
high - Read `AGENTS.md`, `src/lib.rs`, every `src/**/*.rs` file, `README.md`, migrations, and tests; ran `cargo clippy -p convergio-bus --all-targets -- -D warnings`.
