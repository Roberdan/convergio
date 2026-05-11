# Audit — `convergio-planner`

`convergio-planner` is small and clippy-clean, with no unsafe code or production unwrap/expect chains.
The main correctness risk is weak validation/parsing around Opus JSON before durable persistence.
The main documentation drift is that the README still describes only the legacy deterministic line-split behavior.

## Bugs / smells
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| medium | src/opus.rs:81 | `extract_json_object` counts braces inside JSON strings, so valid model output containing `{` or `}` in titles/descriptions can be truncated or rejected. | Extract the JSON object with `serde_json::Deserializer` from the first object start instead of byte-level brace counting. |
| medium | src/schema.rs:81 | `PlanShape::validate` rejects invalid waves but not `sequence < 1`, despite the schema documenting sequence as 1-indexed before persistence. | Validate `sequence >= 1` and reject invalid task ordering before creating durable tasks. |

## Code↔doc drift
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| medium | crates/convergio-planner/README.md:5 | README says `Planner::solve` turns a newline-separated mission into one deterministic task per non-empty line, but default mode is Opus with heuristic only as fallback/override. | Update README to describe `auto`/Opus default and the heuristic fallback mode. |
| low | crates/convergio-planner/AGENTS.md:10 | The invariant says not to embed provider-specific prompts, but `build_prompt` names vendor CLIs and recommends specific providers/models. | Reword the invariant to allow the current reference Opus backend or move provider-specific planning into a capability. |

## Refactor / optimization
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| low | src/opus.rs:1 | `opus.rs` is 247 lines and combines prompting, process spawn, parsing, persistence, and tests. | Split parsing/prompt helpers into a sibling module before adding more Opus behavior. |

## Constitution compliance
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| medium | src/schema.rs:81 | Zero-tolerance invariant risk: invalid task sequences can be accepted from model output and persisted. | Enforce all documented task-order invariants before calling durability writes. |
| low | src/error.rs:9 | P5/i18n risk: planner errors are hardcoded English and may surface through CLI/server callers. | Keep stable error codes in this crate or localize user-facing rendering at the caller boundary. |

## Confidence
high — Read `AGENTS.md`, `README.md`, `src/lib.rs`, all 6 `src/**/*.rs` files, and ran `cargo clippy -p convergio-planner --all-targets -- -D warnings` cleanly.
