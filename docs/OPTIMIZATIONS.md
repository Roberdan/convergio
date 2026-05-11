# Convergio operator-environment optimizations — 2026-05-11

Catalogue of the seven optimizations shipped after the Claude Code
insights audit. Each entry has the same shape:

- **Problem** — the friction pattern this addresses.
- **Where it lives** — file path / PR / commit.
- **How to maintain** — what to do when it breaks or evolves.

The audit ran against 21 Claude Code sessions over three weeks and
flagged four recurring failure modes:

1. Premature "done" claims while PRs were still open or CI was red.
2. Permission prompts breaking autonomous flows mid-execution.
3. Bash:Edit ratio of 5:1 — repeated git/gh/cargo sequences.
4. Output-token-limit errors that lost 5 entire sessions.

The seven optimizations below close all four. They split cleanly
into three operator-side artifacts in `~/.claude/` and four
convergio-side artifacts in this repo, but they are designed to
work as a single contract.

---

## 1. Global allowlist — `~/.claude/settings.json`

**Problem.** Permission prompts blocked git push, gh pr, cargo
install, launchctl, and codesign in the middle of autonomous PR
sessions. The operator had to manually approve each call (and
explicitly complained: "e fallo cazzo che cazzo mi chiedi
permesso").

**Where it lives.** `~/.claude/settings.json`, key
`permissions.allow`. Patterns are `Bash(<glob>)` style.

**How to maintain.**

- Add a new entry whenever a routine command repeatedly triggers a
  permission prompt during autonomous work. Example: when a new
  `gh release` subcommand becomes common, add `Bash(gh release *)`
  to the list.
- Never add `Bash(rm *)` or `Bash(sudo *)` — destructive operations
  must keep prompting. The allowlist is for *idempotent* +
  *reviewable* commands.
- If a permission prompt fires unexpectedly, audit which pattern
  *should* have matched and tighten the glob.

## 2. Stop-hook pre-completion gate — `~/.claude/hooks/pre-completion-gate.sh`

**Problem.** Multiple sessions ended with "done" reported while PRs
were still open, CI was red, or zombie processes lingered. The
operator pushed back ("no se le pr aperte non e finito"); the loop
was structural, not malicious.

**Where it lives.**

- Hook script: `~/.claude/hooks/pre-completion-gate.sh` (executable,
  ~50 lines bash).
- Wired in `~/.claude/settings.json` under `hooks.Stop`.

The script silently exits 0 when the workspace is clean. When
anything would make a "done" claim premature it emits one line per
finding, prefixed `⚠️  PRE-COMPLETION:`. Today it checks:

1. Uncommitted changes in the current git repo.
2. Open PRs returned by `gh pr list`.
3. Orphan `agent-*` worktrees under `.claude/worktrees/`.
4. Live `copilot --allow-all` processes.
5. `agent_processes.status='running'` rows in `state.db` whose PID
   no longer answers `kill -0`.

**How to maintain.**

- Add a new check when a new "leftover" class shows up. Keep each
  check short (single command + `emit`) and non-blocking — exit
  must stay 0.
- If a check is too noisy in solo-human sessions, gate it behind a
  repo-name match (e.g. only fire the convergio-specific checks
  inside `~/GitHub/convergio`).

## 3. Convergio Definition of Done — `AGENTS.md`

**Problem.** "Done" was an opinion. Different sessions had
different bars. Premature completion was inevitable while the bar
was implicit.

**Where it lives.** New section in
[`AGENTS.md`](../AGENTS.md#definition-of-done), shipped in PR #298.

**How to maintain.**

- Treat the 7-point checklist as **append-only**. Removing a point
  is a CONSTITUTION-level change.
- When a new structural failure mode appears (e.g. a new external
  system that must be in sync before "done"), append a point with
  the verification command inline.
- Cross-references to the operator-side `~/.claude/hooks/...` and
  the `/ship-pr` skill must stay in sync — when one of them
  changes path or name, update the doc.

## 4. Lefthook post-merge fleet-cleanup — `lefthook.yml` + `scripts/post-merge-fleet-cleanup.sh`

**Problem.** Autonomous PR sessions left behind stale local
`agent/*` branches, orphan worktrees, and `agent_processes` rows
whose PID was dead. After the F2 fleet sprint the operator counted
13 stale branches and 6 zombies.

**Where it lives.**

- Hook step in [`lefthook.yml`](../lefthook.yml) under
  `post-merge.commands.fleet-cleanup`.
- Script at [`scripts/post-merge-fleet-cleanup.sh`](../scripts/post-merge-fleet-cleanup.sh)
  (executable, ~60 lines bash).

Runs after every `git merge` / `git pull` into the operator's main
checkout. Idempotent and best-effort.

**How to maintain.**

- The script is the single source of truth — `cvg fleet cleanup`
  (optimization #6) mirrors the same logic in Rust.
- When a new class of fleet residue shows up (e.g. a new state file
  in `~/.convergio/`), add a step here first, then port to the CLI.
- Never make the script blocking. The hook is `|| true` for a
  reason: post-merge must complete even if convergio daemon is
  down.

## 5. Global `/ship-pr` skill — `~/.claude/skills/ship-pr/SKILL.md`

**Problem.** The push → poll CI → merge → cleanup sequence took
5-7 hand-rolled Bash calls per PR. The 2026-05 audit measured a
5:1 Bash:Edit ratio. Each call was also a chance to skip a step
and claim premature completion.

**Where it lives.** `~/.claude/skills/ship-pr/SKILL.md`
(~120 lines markdown). Invoked as `/ship-pr` or by the trigger
phrases "ship this", "merge this", "send the PR".

**How to maintain.**

- The skill's "Contract" section is the prose form of the
  Definition of Done from optimization #3. When the DoD evolves,
  update the skill.
- Convergio-specific extensions (e.g. `cvg task close-post-hoc`
  after merge) live in their own section so the skill stays
  reusable for non-convergio repos.

## 6. `cvg fleet cleanup` — `crates/convergio-cli/src/commands/fleet_cleanup.rs`

**Problem.** Operators wanted the same cleanup the lefthook
post-merge hook runs, but on demand and with a structured report
(JSON, human, plain). Without it, ad-hoc cleanup meant remembering
five `git` incantations.

**Where it lives.** PR #299. Subcommand of `cvg fleet`:

```
$ cvg fleet cleanup --dry-run
cvg fleet cleanup — would remove 0 orphan worktree(s), 0 stale branch(es).
  note: agent_processes rows in state.db are reconciled by the daemon reaper.
```

Mirrors `scripts/post-merge-fleet-cleanup.sh` but stays on the
operator side — DB writes are explicitly out of scope per
`crates/convergio-cli/AGENTS.md` ("Do not import server crates or
write SQLite directly"). DB-side reconciliation is left to the
daemon reaper.

**How to maintain.**

- Keep the CLI and the shell script in lock-step. The two unit
  tests (`sweep_on_clean_repo_reports_nothing`,
  `sweep_finds_local_agent_branch_with_no_remote`) lock in the
  contract.
- Output schema (`OutputMode::Json`) is stable: external tools may
  parse it. Field renames are a breaking change.

## 7. `PrLinkGate` in `convergio-durability` — `gates/pr_link_gate.rs`

**Problem.** The audit chain ended with `task.done` rows whose
provenance was "trust me bro" — no anchor that the work was
actually shipped. The full pre-merge contract (PR merged on
`origin`, branch deleted, no orphan worktree) needs network access
and cannot run inside a synchronous gate.

**Where it lives.** PR #300.
[`crates/convergio-durability/src/gates/pr_link_gate.rs`](../crates/convergio-durability/src/gates/pr_link_gate.rs).
Registered last in `gates::default_pipeline()`.

The gate is in-process and minimal:

- Fires only on `target_status == Done`.
- Refuses with `pr_link_missing` if no `plan_pr_links` row exists
  for the task's plan.
- Visible through `GET /v1/gates/preconditions` (P3-2).

Remote-state checks (PR merged, branch deleted) stay
operator-side: that's what optimizations #2 (pre-completion gate)
and #4 (post-merge cleanup) are for. The three together are the
full pre-merge contract.

**How to maintain.**

- When a new transition needs PR provenance (e.g. a future
  `task.shipped` state), extend the gate's `target_status` match.
- Coupling with `plan_pr_links` is intentional: if the schema
  evolves (column rename, new constraint), both the gate's query
  and the CLI flow (`cvg pr link`, `cvg task transition --pr-url`)
  must be updated together.
- The gate is **strictly stricter** than the previous default
  pipeline. Old plans that closed tasks without a PR row will need
  a backfill or an explicit allowlist — add a special case here if
  history pre-dates the schema.

---

## Cross-references

| Failure mode flagged in the 2026-05 audit | Optimization(s) that close it |
|---|---|
| Premature "done" claims | #2 pre-completion gate, #3 DoD section, #5 `/ship-pr` skill, #7 `PrLinkGate` |
| Permission prompts breaking autonomous flow | #1 allowlist |
| Bash:Edit 5:1 ratio | #5 `/ship-pr` skill, #6 `cvg fleet cleanup` |
| Stale branches / zombie processes | #4 post-merge hook, #6 `cvg fleet cleanup` |
| Output token-limit errors | #8 long-output → file memo (operator-side) |
| Tasks pane drowned by terminal rows | #9 TUI hide-terminal filter (default ON) |
| Trial-and-error on macOS internals | #10 trial-and-error → web memo (operator-side) |

## 8. Long-output → file memo

**Problem.** ≥5 audited sessions hit the 500-token output cap and
lost content; dense table replies prompted "non ho capito ridimmi
bene". Inline long outputs also burn the chat context they live in.

**Where.** Operator-side global memory at
`~/.claude/projects/-Users-Roberdan-GitHub/memory/feedback_long_output_to_file.md`,
indexed in `MEMORY.md`. Backed by `~/.claude/reports/` for the
generated files.

**How.** When a reply would carry more than ~50 rows or ~2 KB
(multi-PR status, audit reports, dense tables, plan dumps), write
the full content to `~/.claude/reports/<YYYY-MM-DD>-<slug>.md` and
reply with: 1-line summary + counts + absolute file path. The chat
reply still answers the question — the file is durable detail, not a
substitute.

**Maintain.** Memory persists across sessions; nothing in this repo
to maintain. If the rule needs to be enforced at the daemon CLI
level later (e.g. a `--output-to-file` flag on `cvg status`,
`cvg pr stack`, `cvg fleet ls`), open a follow-up plan.

## 9. TUI hide-terminal task filter (default ON)

**Problem.** The `cvg dash` Tasks pane showed every status side-by-
side, so `done`/`failed` rows drowned out live work as plan history
grew.

**Where.** `crates/convergio-tui/src/state.rs` (the
`show_terminal_tasks` flag), `crates/convergio-tui/src/panes/tasks.rs`
(filter at render time, title carries `(active/total · t:show all)`),
`crates/convergio-tui/src/keymap.rs` (`t` toggles the new
`Action::ToggleShowTerminalTasks`).

**How.** Default state hides `done`/`failed`; press `t` to reveal
them. The underlying `state.tasks` Vec is intact so scope filters,
detail mode, and other panes still see everything — only the
Tasks-pane view is filtered. PR #303.

**Maintain.** When new task statuses land in `convergio-durability`,
review the `is_terminal` helper in `panes/tasks.rs` to decide whether
they count as terminal for the filter.

## 10. Trial-and-error → web memo (operator-side)

**Problem.** During the May 2026 macOS Tahoe Spotlight saga the
user cut off a trial-and-error loop ("non possiamo andare a
tentativi"); the root cause (a corrupted plist, 22 consecutive
crashes) only surfaced after switching to web search + external model
consult. Trial-and-error on system internals also leaves zombie
procs, half-applied configs, and killed daemons in its wake.

**Where.** Operator-side global memory at
`~/.claude/projects/-Users-Roberdan-GitHub/memory/feedback_no_trial_and_error.md`,
indexed in `MEMORY.md`.

**How.** On system/infra bugs (launchd, code-signing, Spotlight,
SQLite locking, daemon spawn loops, kernel/macOS internals), after
the 2nd failed fix attempt stop and consult web/docs/crash logs
before attempt #3. Routine application bugs (compile errors, unit
test failures) do not trigger the rule.

**Maintain.** Memory persists across sessions; nothing in this repo
to maintain. If the same pattern shows up inside Convergio dispatch
loops (e.g. an executor retrying a spawn that keeps failing), encode
the same back-off-and-research behaviour there as a follow-up.

## On the horizon

Patterns the insights flagged that are **not** addressed here:

- **`--output-to-file` flag on heavy `cvg` commands** (status, pr
  stack, fleet ls). Memo #8 handles it operator-side; a daemon-side
  flag would make it discoverable for new agents too.
- **Deploy-verify-or-rollback loop.** Out of scope for convergio
  (we don't deploy a production app); relevant for the
  MirrorHR-style workflows.

## Maintenance routine

When a new optimization is added, append a section here with the
same six headings (Problem / Where / How to maintain). When an
optimization is retired, mark it `## ~~N. <name>~~ (retired
YYYY-MM-DD)` instead of deleting it — the audit trail of why a
guardrail was removed is as important as the guardrail itself.
