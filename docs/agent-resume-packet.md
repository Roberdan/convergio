# Agent resume packet

**This is the file a fresh AI agent should read first** when handed
this repository. It is paste-ready: every command line below works
verbatim against the running daemon and the current `cvg` binary.
The packet is the timeless protocol — it must NOT cite version
numbers, PR numbers, or finding IDs that rot. For live state, run
`cvg session resume`.

---

## 1. Identity

You are operating on a Mac at `/Users/Roberdan/GitHub/convergio`.

The Convergio daemon listens on `http://127.0.0.1:8420` and is the
**source of truth** for plans, tasks, evidence, and the hash-chained
audit log. If it is down:

```bash
cvg service start
cvg health    # expect ok=true, service=convergio
```

Your durable agent identity in `agent_registry` is
`claude-code-roberdan`. Use it on every transition:

```bash
cvg task transition <task_id> in-progress --agent-id claude-code-roberdan
```

If the registry has lost the row (fresh DB), re-register via the
typed action (do **not** call HTTP directly):

```bash
cvg agent register --id claude-code-roberdan --kind claude \
  --name "Claude Code (Roberdan local)" --host macOS \
  --capability code,test,doc,rust,bash,markdown
```

## 2. Cold-start reads (in order)

Live state first — every value below is a daemon query, never stale:

```bash
cvg session resume                # daemon, audit, active plan, next tasks, open PRs
cvg session resume --output json  # same brief, machine-readable
cvg pr stack                      # merge order + conflict matrix (uses gh)
git log --oneline main -10        # what landed recently
```

Then the timeless reference set:

```bash
cat AGENTS.md                                     # cross-vendor agent rules
cat CONSTITUTION.md                               # non-negotiables
cat ROADMAP.md                                    # current waves and priorities
cat docs/INDEX.md                                 # auto-generated file map (Tier-1 retrieval)
cat docs/agent-protocol.md                        # MCP tool loop
cat docs/multi-agent-operating-model.md           # how swarms use Convergio
```

Drill into a single ADR or plan only when the task demands it.

## 3. Worktree discipline (CONSTITUTION § 15)

If another agent might be operating on this repo at the same time,
work from a separate git worktree.

```bash
git worktree add .claude/worktrees/<branch> -b <branch>
cd .claude/worktrees/<branch>
# work, commit, push as usual
gh pr create --base main --head <branch> --title "..." --body "..."
# at end of work
cd /Users/Roberdan/GitHub/convergio
git worktree remove .claude/worktrees/<branch>
```

`.claude/worktrees/` is gitignored AND excluded from `.claudeignore`,
`.cursorignore`, and `.github/copilot-ignore`, so worktrees stay off
`git status`, off agent context windows, and out of editor search.

## 4. Workspace lease pattern (claim before edit)

When editing a file other agents might race against, claim a lease
through the typed action:

```bash
cvg workspace lease claim --resource file:convergio-local:<path> \
  --agent-id claude-code-roberdan --purpose "<why>" --expires-in 1h
# ... edit the file ...
cvg workspace lease release <lease_id>
```

For solo sessions this is overhead you may skip. The lease pattern
exists for the multi-agent future and the merge-arbiter (ADR-0007).

## 5. Required local pipeline before any push

```bash
cargo fmt --all -- --check
RUSTFLAGS="-Dwarnings" cargo clippy --workspace --all-targets -- -D warnings
RUSTFLAGS="-Dwarnings" cargo test --workspace
./scripts/check-context-budget.sh              # exit 0 clean, 2 soft-warn ok
./scripts/generate-docs-index.sh --check
./scripts/legibility-audit.sh --quiet          # target ≥ 70, ideal ≥ 85
```

If any step fails, fix first, then re-run **all** of them. Never
push with known failures.

## 6. PR template hygiene (CONSTITUTION § 13)

Every PR body has these sections (CI-enforced):

```markdown
## Problem
## Why
## What changed
## Validation
## Impact
```

Files-touched manifests are produced by:

```bash
git diff --name-only main...HEAD
```

`cvg pr stack` cross-checks the manifest against the real diff and
surfaces `Mismatch` / `Missing` if you got it wrong.

## 7. WIP commit message protocol

If you must pause work, follow [`docs/wip-commit-template.md`](./wip-commit-template.md).
The commit body must include: files modified with `wc -l`, new
module declarations required, build/test state at pause, the resume
checklist, and the canonical resume command (`git checkout <branch>`
then `git rebase origin/main`).

## 8. Constitution touchstones

| § | What it says | Common mistake |
|---|--------------|----------------|
| § P5 | i18n first — strings flow through Fluent | new CLI command shipped EN-only |
| § 6 | clients propose, daemon disposes; only Thor sets `done` | calling `cvg task transition X done` (clap blocks at parse) |
| § 11 | every crate has AGENTS.md + CLAUDE.md | new crate shipped without one |
| § 13 | per-file 300 lines, per-crate LOC caps | new file lands at 301 lines |
| § 15 | parallel-agent work uses worktrees | one shared checkout, two agents |
| § 16 | legibility score ≥ 70 / 100 | regression during a busy PR wave |

## 9. The first wave for a new session

The user's standing ask: any new session opens with a repo
optimisation pass — make the codebase more legible for the next
agent before adding new surface. The concrete queue lives in the
daemon, not here:

```bash
cvg session resume     # next-priority pending tasks, ordered by wave/sequence
```

Standing principle:

- *Housekeeping first* — install-script, hooks, locale pins, WIP protocol.
- *Then retrieval* — frontmatter, coherence checks, file-map quality.
- *Then architecture* — splitting near-cap crates (check
  `./scripts/legibility-audit.sh` for current LOC).

## 10. What to do when stuck

1. Stop. Do not guess your way through.
2. Check `cvg status --project convergio-local` — the queue is the
   single source of truth for "what is open".
3. Read the most recent friction log in `docs/plans/` — your problem
   is probably already named there.
4. If genuinely new, capture it as a new finding (next number after
   the last F##) and continue.
5. If a hard architectural fork emerges, write an ADR draft at
   `docs/adr/00NN-<title>.md` (status `proposed`) and stop until
   the user reviews.

The audit chain accepts every refusal. Convergio's loyalty is to
the truth, not to the agent's pace.
