# Audit — `convergio-graph`

`convergio-graph` is generally healthy: clippy is clean and production code avoids unsafe, panic macros, and unwrap/expect chains.
The main concrete bug is ADR mention IDs that cannot resolve because docs are stored with file paths in their node identity while mention edges omit the path.
The largest gap is documentation drift in `AGENTS.md`, especially the missing `refresh.rs` and unimplemented lazy-on-read refresh contract.

## Bugs / smells
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| medium | src/doc_link.rs:54 | `related_adrs` mention edges compute ADR node ids with `file_path: None`, but `parse_doc` creates ADR node ids with `Some(rel_path)`, so related-ADR edges cannot resolve. | Resolve related ADRs by discovered rel path or make ADR node identity path-independent. |
| low | src/build.rs:130 | Unknown doc edges are swallowed at debug level, so broken doc-link invariants can disappear during normal graph builds. | Count and surface skipped doc edges in `BuildReport` or at least warn with source/destination context. |
| low | src/cluster_io.rs:96 | `file_loc` silently returns `0` for every I/O failure, making deleted/unreadable files indistinguishable from empty files. | Return `Result<u64>` and let `cluster_for_crate` report skipped files explicitly. |

## Code↔doc drift
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| medium | crates/convergio-graph/AGENTS.md:18 | The invariant says reads lazily compare mtimes and re-parse stale nodes inline, but query paths only read stored rows and do not invoke parsing. | Update the invariant as future work or implement lazy refresh in query/store paths. |
| medium | crates/convergio-graph/AGENTS.md:20 | The invariant names `CONVERGIO_GRAPH_REFRESH_SECS`, but no graph refresh loop or environment-variable usage exists in the crate/server. | Remove the shipped-behavior claim or add the opt-in refresh loop. |
| medium | crates/convergio-graph/AGENTS.md:35 | The module layout lists `refresh.rs`, but the crate has no `src/refresh.rs` module. | Delete the row or add the missing module. |
| low | crates/convergio-graph/AGENTS.md:33 | `parse.rs` is documented as parsing a crate root, but `parse_file` only parses one Rust file and crate walking lives in `build.rs`. | Reword the module layout to "single `*.rs` file". |
| low | crates/convergio-graph/AGENTS.md:34 | `doc_link.rs` is documented as grep-based ADR↔crate edges, but implementation only parses YAML frontmatter lists. | Reword the doc to frontmatter-based ADR/doc edges. |
| low | crates/convergio-graph/AGENTS.md:36 | `model.rs` is documented as owning `ContextPack`, `DriftReport`, and `ClusterReport`, but those live in `query.rs`, `drift.rs`, and `cluster.rs`. | Update the module layout ownership table. |
| low | crates/convergio-graph/AGENTS.md:40 | The tests section says E2E tests use `tests/fixtures/`, but the crate has integration tests without a fixtures directory. | Update the tests guidance to match the current integration-test shape. |

## Refactor / optimization
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| low | src/parse.rs:1 | `parse.rs` is 284 lines and close to the 300-line cap. | Split `flatten_use`/path formatting or visitor tests into a sibling module before adding parser features. |
| low | src/drift.rs:1 | `drift.rs` is 277 lines and close to the 300-line cap. | Move test fixtures/helpers or git-diff resolution helpers into a sibling module. |

## Constitution compliance
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| medium | crates/convergio-graph/AGENTS.md:18 | P1/no-scaffolding risk: lazy-on-read and refresh-loop behavior is documented as shipped but not implemented. | Align docs with shipped behavior or wire the missing behavior end-to-end. |
| low | src/lib.rs:34 | P2/security is positive: the crate forbids unsafe code and no unsafe blocks were found. | Keep `#![forbid(unsafe_code)]` in place. |
| low | src/query.rs:120 | P2/resource safety is positive: query input and token expansion are capped before database fan-out. | Preserve the input/token caps when extending matching. |
| low | src/lib.rs:1 | P3/P5 are not materially exercised because this is a library crate with no direct UI strings. | Keep user-facing rendering localized in CLI/server layers. |

## Confidence
high — Read `AGENTS.md`, `src/lib.rs`, all 16 `src/**/*.rs` files, integration tests, route/CLI references, and ran `cargo clippy -p convergio-graph --all-targets -- -D warnings`.
