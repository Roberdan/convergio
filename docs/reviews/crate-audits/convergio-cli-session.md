# Audit — `convergio-cli-session`

Clippy is clean for `convergio-cli-session`.
The crate is mostly well-factored, with no production `unwrap`/`expect` or `unsafe` found.
Main risks are stale scaffold docs, remaining pre-stop stubs, and hardcoded English in user-facing paths.

## Bugs / smells
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| low | src/heartbeat_since_last_turn.rs:53 | A failed heartbeat POST still rewrites the throttle timestamp, hiding repeated heartbeat failures for one interval. | Only update the timestamp after a successful POST, while still swallowing the outward error. |

## Code↔doc drift
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| medium | src/pre_stop.rs:3 | The module doc still describes a stub scaffold even though three checks are implemented. | Rewrite the module doc to describe the mixed implemented/stub registry. |
| low | src/checks/mod.rs:6 | The doc says plan↔PR drift stays stubbed, but `check_1_plan_pr_drift` is a real registered check. | Update the module doc to list only the checks that still remain stubbed. |
| low | crates/convergio-cli-session/AGENTS.md:26 | The boundary says pre-stop shell-outs are `gh` and `git`, but `check_1_plan_pr_drift` also shells out to `curl`. | Mention `curl` or move the daemon call behind the injected client when the trait becomes async. |

## Refactor / optimization
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| low | src/register_and_poll.rs:296 | The file sits four lines below the 300-line cap, so routine additions will immediately force a split. | Move tests or helper functions into a sibling module before adding behavior. |
| low | src/checks/check_1_plan_pr_drift.rs:274 | The check is close to the 300-line cap and mixes shelling, parsing, extraction, HTTP, and tests. | Split parser/extractor tests or HTTP status lookup into a sibling helper module. |

## Constitution compliance
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| medium | src/pre_stop_run.rs:38 | Human pre-stop output is hardcoded English instead of `convergio-i18n` Fluent keys. | Route human strings through `convergio-i18n` with `session-*` keys. |
| medium | src/heartbeat_since_last_turn.rs:50 | The first-call stderr message is hardcoded English in a user-visible hook path. | Add a localized key or suppress the message in non-human hook mode. |
| medium | src/pre_stop.rs:118 | Three registered pre-stop checks still return `NotImplemented`, which is shipped scaffold in a safety-net command. | Implement the remaining checks or clearly gate them as future, non-blocking advisory checks. |

## Confidence
high — Read `AGENTS.md`, `README.md`, `src/lib.rs`, every `src/**/*.rs` file, and ran crate-scoped clippy.
