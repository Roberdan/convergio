# Audit — `convergio-cli-pr`

`convergio-cli-pr` stays within crate boundaries and clippy is clean.
The main correctness risk is post-merge metadata/audit failures being converted into notes or ignored.
The main constitution issue is hardcoded English human output outside the localized `stack` renderer.

## Bugs / smells
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| medium | crates/convergio-cli-pr/src/pr_merge.rs:122 | `merge_record` evidence failures after a successful merge are only appended to `notes`, so the command exits successfully with missing audit metadata. | Return a non-zero partial-failure error or require explicit operator acknowledgement for failed evidence writes. |
| low | crates/convergio-cli-pr/src/pr_sync.rs:107 | `plan_pr_links` POST failures are discarded even though the comment says they are logged. | Record the failure in `SyncReport.failed` or emit a visible note. |
| low | crates/convergio-cli-pr/src/pr.rs:87 | Per-PR diff fetch failures fall back to manifest-only analysis with no visible warning. | Surface a manifest status or report note when `gh pr view --json files` fails. |

## Code↔doc drift
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| medium | crates/convergio-cli-pr/src/pr.rs:49 | The command docs promise an "8-check" merge pre-flight, but `eight_check` currently evaluates four entries. | Rename the claim to four checks or implement the missing four checks. |
| low | crates/convergio-cli-pr/src/pr_merge_io.rs:2 | The module doc says AUTO-block auto-resolve lives in `pr_merge_resolve`, but no such module exists in this crate. | Mark auto-resolve as future work or add the module when implemented. |
| low | crates/convergio-cli-pr/src/lib.rs:1 | The entry-point doc lists `stack`, `sync`, and `merge`, but omits the shipped `link` and `who` subcommands. | Update the crate summary to include all `cvg pr` subcommands. |

## Refactor / optimization
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| low | crates/convergio-cli-pr/src/pr_sync.rs:277 | `pr_sync.rs` is close to the 300-line cap and mixes GitHub fetch, daemon mutations, and report rendering. | Split rendering or GitHub fetch helpers into a sibling module before adding behavior. |
| low | crates/convergio-cli-pr/src/pr_merge.rs:271 | `pr_merge.rs` is close to the 300-line cap and contains orchestration, report rendering, and evidence payload construction. | Move rendering or payload construction into a sibling module. |

## Constitution compliance
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| medium | crates/convergio-cli-pr/src/pr_merge.rs:196 | P5 i18n is violated because `cvg pr merge` human output is hardcoded English. | Route human strings through `convergio_i18n::Bundle` or document why the command is intentionally unlocalized. |
| medium | crates/convergio-cli-pr/src/pr_sync.rs:230 | P5 i18n is violated because `cvg pr sync` human output is hardcoded English. | Route human strings through `convergio_i18n::Bundle` before extending the command. |
| medium | crates/convergio-cli-pr/src/pr_link.rs:61 | P5 i18n is violated because `cvg pr link` human output is hardcoded English. | Localize the success message or restrict it to plain/script output. |
| medium | crates/convergio-cli-pr/src/pr_who.rs:62 | P5 i18n is violated because `cvg pr who` human output is hardcoded English. | Localize the empty and ownership messages. |

## Confidence
high — Read `AGENTS.md`, `src/lib.rs`, every `src/**/*.rs` file, crate `README.md`, the CLI shim, and the PR template; ran `cargo clippy -p convergio-cli-pr --all-targets -- -D warnings`.
