# Audit — `convergio-tui`

`convergio-tui` is functional and crate-scoped clippy is clean with `-D warnings`.
The main risks are swallowed refresh/`gh` errors that make stale or empty data look healthy.
The biggest drift is documentation still describing the older 4-pane/MVP dashboard while code now ships Bus and drill-down behavior.

## Bugs / smells
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| medium | crates/convergio-tui/src/client.rs:96 | `/v1/plans` failures are coerced to an empty plan list, so `snapshot_core` can return `Ok` and mark the footer connected after a daemon/API failure. | Propagate the core plan fetch error or carry a partial-snapshot status that renders as degraded. |
| medium | crates/convergio-tui/src/client.rs:186 | Per-plan task fetch failures are silently converted to empty task lists. | Preserve per-plan fetch errors in the snapshot and surface a degraded/error indicator. |
| low | crates/convergio-tui/src/client_gh.rs:68 | `gh pr list` spawn, auth, and non-zero exit failures all render as an empty PR list. | Distinguish "no PRs" from "gh unavailable/failed" in `PrSummary` state or the pane title. |
| low | crates/convergio-tui/src/bus_stream.rs:103 | Failure to build the bus stream HTTP client exits the supervisor without updating transport state. | Set `Transport::Reconnecting` or an explicit error state before returning. |

## Code↔doc drift
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| medium | crates/convergio-tui/README.md:3 | README still claims a single 4-pane console, but `Pane::ALL` and rendering include five panes with Bus. | Update README layout/status text to include the Bus pane. |
| medium | crates/convergio-tui/README.md:95 | README labels Enter drill-down as future work, but `keymap.rs` and `navigation.rs` implement it today. | Move drill-down from "Next" to shipped behavior and document `Enter`/`Esc`. |
| low | crates/convergio-tui/AGENTS.md:55 | Module layout documents `run(daemon_url, tick_secs)`, but `lib.rs` exposes `run(daemon_url, tick_secs, github_slug)`. | Refresh the module table signature. |
| low | crates/convergio-tui/AGENTS.md:58 | AGENTS says `tick.rs` owns the interval refresh loop, but `lib.rs` owns the interval and `tick.rs` only has pure helpers. | Update the table to describe `tick.rs` as formatting/clamp helpers. |
| low | crates/convergio-tui/AGENTS.md:83 | AGENTS says `tests/client.rs` exercises the reqwest client, but no such test file exists. | Replace the test claim with the actual integration tests or add the missing test later. |

## Refactor / optimization
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| low | crates/convergio-tui/src/scope.rs:298 | `scope.rs` has two lines of headroom under the 300-line cap. | Split PR/bus scope helpers or shared set-building into a sibling module before extending. |
| low | crates/convergio-tui/src/state.rs:293 | `state.rs` has seven lines of headroom and still owns pane/cursor/connection/core state types. | Move cursor or connection types into focused modules before adding fields. |
| low | crates/convergio-tui/src/lib.rs:286 | `lib.rs` is close to the cap while owning terminal setup, event loop, key dispatch, and refresh spawning. | Extract terminal setup/teardown or progressive refresh orchestration. |
| low | crates/convergio-tui/src/panes/plans.rs:170 | Unicode-safe truncation helpers are duplicated across multiple pane modules. | Centralize `short`/`truncate` helpers in a small text utility module. |

## Constitution compliance
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| medium | crates/convergio-tui/src/client.rs:96 | Zero-tolerance observability is weakened because core daemon failures can render as a successful empty dashboard. | Make failed core fetches visible as disconnected/degraded instead of success-shaped empties. |
| low | crates/convergio-tui/src/theme.rs:5 | The accessibility style module says no `Color::DarkGray` literal exists elsewhere, but `header_banner.rs` uses it for stats. | Route header stat styling through `theme::dim()` or update the style invariant. |
| low | crates/convergio-tui/src/plan_counts.rs:26 | The crate-level "no business logic" invariant is violated by client-side task count aggregation and pane-derived plan stats. | Either expose these summaries server-side or relax the invariant to allow presentation-only aggregation. |

## Confidence
high — Read crate AGENTS, README, `src/lib.rs`, and every `src/**/*.rs`; ran `cargo clippy -p convergio-tui --all-targets -- -D warnings`.
