# Audit — `convergio-runner`

The crate is focused, pure-preparation oriented, and crate-scoped clippy is clean.
The main issue is a security/doc mismatch around Copilot deny-list enforcement for permissive profiles.
No raw HTTP calls, unsafe code, or non-test panic chains were found.

## Bugs / smells
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| high | crates/convergio-runner/src/runner/mod.rs:184 | Copilot `Sandbox` emits `--allow-all` without the destructive-command `--deny-tool` list that `PermissionProfile::copilot_deny_tools` says is always applied. | Apply the deny-list in every Copilot profile or make the bypass explicit and remove the false invariant. |
| high | crates/convergio-runner/src/runner/mod.rs:187 | Copilot `Unrestricted` emits `--allow-all` and `--add-dir` without any `--deny-tool` entries, despite comments saying daemon-side deny-list still applies. | Add the deny-list to `Unrestricted` or route unrestricted launches through an audited daemon-side command filter. |

## Code↔doc drift
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| medium | crates/convergio-runner/src/profile.rs:152 | The doc says Copilot `--deny-tool` patterns are always applied, even on `Sandbox`, but `runner.rs` skips them for `Sandbox` and `Unrestricted`. | Align the implementation with the documented invariant or narrow the doc to Standard/ReadOnly only. |
| low | crates/convergio-runner/src/runner/mod.rs:180 | The comment says ADR-0033 replaces `--allow-all-tools` with a per-tool whitelist, but Standard still adds `--allow-all-tools`. | Update the comment to match the non-interactive Copilot behavior documented later in the same branch. |

## Refactor / optimization
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| low | crates/convergio-runner/src/runner/mod.rs:1 | `runner.rs` is 285 lines and mixes dispatch, Claude argv, Copilot argv, PATH checks, and tests near the 300-line cap. | Split vendor-specific runners into `runner/claude.rs` and `runner/copilot.rs` or move PATH checks to a helper module. |

## Constitution compliance
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| high | crates/convergio-runner/src/runner/mod.rs:184 | `Sandbox` bypasses Copilot's tool gates without the documented destructive-command deny-list, weakening the security-first principle. | Enforce the deny-list even for sandboxed Copilot runs unless an explicit sealed-environment override is audited. |
| high | crates/convergio-runner/src/runner/mod.rs:187 | `Unrestricted` bypasses Copilot's tool gates without deny-list flags while comments imply destructive commands remain blocked. | Preserve destructive-command refusals under `Unrestricted` or rename it to a fully unsafe profile with explicit audit controls. |

## Confidence
high — Read `AGENTS.md`, `src/lib.rs`, every `src/**/*.rs` file, the runner argv tests, grepped for panic/unsafe/debt markers, cross-checked docs against code, and ran crate-scoped clippy cleanly.
