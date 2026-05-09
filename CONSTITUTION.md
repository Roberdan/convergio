# Convergio Constitution

These rules keep Convergio focused: a local runtime that refuses
low-quality AI-agent work before it is marked done.

---

# Sacred principles

## P1. Zero tolerance for technical debt, errors and warnings — *enforced*

In any language, in any output an agent attaches as evidence of work
done. No `TODO`, no `FIXME`, no `unwrap()`, no `console.log`, no
`pdb.set_trace`, no ignored tests, no `as any`, no `// nolint`, no
debug prints. Build must be clean. Tests must pass. Linters must be
silent.

**Scope clarification.** P1 governs the *content of evidence* an
agent attaches. Production source code in this repo follows the
spirit of the rule: prefer `?` propagation over `unwrap()` /
`expect()`, and reserve panicking constructs for genuinely
infallible load-time setup. Inline `#[cfg(test)] mod tests` and
doc-tests are exempt — they are tests, not evidence.

Operationally: `NoDebtGate`, `ZeroWarningsGate` and `NoSecretsGate`
refuse `submitted`/`done` transitions when evidence carries debt
markers, non-clean quality signals, or common credential leaks.

## P2. Security first, local first — *partial*

Convergio is a single-user localhost daemon. The safe default is:

- SQLite file on the user's machine
- HTTP bind on `127.0.0.1`
- no remote users, accounts, tenants, RBAC, or hosted control plane
- no secrets in evidence, logs, or code
- spawned agents run with the daemon user's privileges and are not a
  sandbox boundary

LLM-specific threats still matter. Prompt-injection patterns, secret
leaks, and suspicious evidence are treated as bugs in the gate surface,
not as "later" concerns. The MVP includes first-pass secret detection;
future security gates may add prompt-injection refusal, but the runtime
remains local-first.

## P3. Accessibility first — *planned*

Accessibility is a principle, not a polish step.

1. Agent output that creates UI must be accessible.
2. Convergio's own CLI must be usable without color, animation, or
   terminal-specific assumptions.

Planned gates may scan UI evidence for common accessibility failures.
Until then, any feature that makes Convergio harder to use with assistive
technology is a bug.

## P4. No scaffolding only — *enforced for self-admitted stubs*

If an agent says "done", the work must actually be reachable from code
or tests. Creating files without wiring them, leaving placeholders, or
shipping skeleton functions is not done.

Operationally: `NoStubGate` refuses `submitted`/`done` transitions when
evidence contains explicit scaffolding markers such as `stub`,
`placeholder`, `to be wired`, `not implemented`, `(skeleton)`, or
language-specific not-implemented constructs.

Planned deeper gates may parse diffs to prove new modules, routes, and
public functions are actually wired.

## P5. Internationalization first — *enforced*

The product must be usable by people who do not read English fluently.
Italian and English are first-class from day one.

Operationally:

- CLI user-facing strings flow through `convergio-i18n`
- Fluent bundles ship for `en` and `it`
- coverage tests assert both locales expose the same keys
- machine-readable API error codes stay stable English identifiers

---

# Technical non-negotiables

## 1. Local SQLite only

The runtime uses one local SQLite database file. No external database is
required or supported in the MVP. This is deliberate: zero setup, low
resource use, reproducible local state, and a small operational surface.

Default database:

```text
sqlite://$HOME/.convergio/v3/state.db?mode=rwc
```

## 2. Cooperate, don't compete

LangGraph, CrewAI, Claude Code skills, AutoGen, shell scripts and custom
Python are clients, not competitors. Convergio gives them local
durability, audit and gates. It does not provide a DSL, agent framework
or workflow language.

## 3. Reference implementation is part of the product

Layer 4 (`planner`, `thor`, `executor`) stays small but usable. It
exists so a new user can start the daemon and see a complete local loop
without writing a client first.

## 4. Anti-feature creep

Deferred or cut unless real local-user adoption proves the need:

- remote deployment
- account or organization model
- RBAC
- hosted service
- mesh / multi-host
- knowledge catalog
- billing
- skills marketplace

## 5. Every feature must close the loop

Every feature has: input -> processing -> output -> feedback -> state
update -> visible result. If the user cannot see the result, it is not
done.

## 6. Server-enforced gates only

A task cannot honestly be marked complete by the client alone. The
daemon verifies evidence and transitions state. Clients propose; the
daemon disposes.

The gate pipeline is fixed and must remain server-side:

```text
plan_status -> evidence -> no_debt -> no_stub -> zero_warnings -> wave_sequence
```

Any new gate must be documented and tested.

**`done` is set only by the validator.** Per [ADR-0011](docs/adr/0011-thor-only-done.md),
`POST /v1/tasks/:id/transition` returns `403 done_not_by_thor` for
`target=done`. The only path from `submitted` to `done` is
`POST /v1/plans/:id/validate` (CLI: `cvg validate <plan_id>`). Each
promotion writes a `task.completed_by_thor` audit row. Agents
propose `submitted`; Thor disposes `done`.

**One narrow operator exception** lives at
`POST /v1/tasks/:id/close-post-hoc` (CLI:
`cvg task close-post-hoc <id> --reason "..."`), introduced by
[ADR-0026](docs/adr/0026-plan-wave-milestone-vocabulary.md). It
exists for triage of tasks whose work shipped outside the
daemon's evidence flow (e.g. before the `Tracks: <uuid>`
convention was adopted). The route requires a non-empty
`reason` that references a shipping artifact (e.g. PR number,
commit SHA, or release tag/link), refuses already-`done` tasks
for idempotency, and writes a `task.closed_post_hoc` audit row.
It is not an agent surface — agents must still walk
`submitted → validate`.

## 7. Audit log is append-only and hash-chained

Every audited state transition writes a row to `audit_log` whose `hash`
is `sha256(prev_hash || canonical_json(payload))`. The chain is
verifiable via `GET /v1/audit/verify`.

Mutating an audit row, or breaking the chain, is a bug.

## 8. CLI is a pure HTTP client

`cvg` must not import server crates. It speaks HTTP to the local daemon.

## 9. Tests are the spec

If behavior is not under test, it is not guaranteed. Public HTTP routes
and library APIs require tests. Bug fixes require regression tests.

## 10. Multi-agent coordination goes through the daemon

Agents must coordinate through Convergio state, not private chats,
sidecar files, or direct SQLite writes.

Allowed coordination channels:

- daemon HTTP API;
- `convergio.act` / `convergio.help`;
- task state and heartbeat;
- evidence;
- hash-chained audit;
- plan-scoped message bus;
- future workspace leases, patch proposals, and merge queue.

An agent may read files it is asked to work on, but durable coordination
state belongs to the daemon.

## 11. Agent context is hierarchical and mandatory

Every crate under `crates/` must contain:

- `AGENTS.md` — crate-local responsibility, boundaries, invariants, tests;
- `CLAUDE.md` — symlink or pointer to the same local guidance.

Every new major folder, protocol surface, or capability must add or
update the nearest `AGENTS.md`. Do not duplicate the root instructions
into subfolders. Local instructions must be short, concrete, and scoped.

Cross-vendor instruction files must not diverge. If Claude, Copilot,
Cursor, or another host needs a special filename, point it at the same
source of truth.

## 12. Plans are durable repository artifacts

Major initiatives must have an agent-executable plan under
`docs/plans/`. Session-local plans are working memory; repo plans are
project history.

Each repo plan must include:

- objective;
- current state;
- invariants;
- phases;
- task IDs;
- dependencies;
- acceptance criteria;
- validation commands;
- links to ADRs and implementation files.

Obsidian may mirror or index repo plans, but the repo plan is the
engineering source of truth for this codebase.

## 13. Agent context budget

Convergio is built to be edited by AI agents. Agents have a finite
context window (Claude 200k, others smaller). Repos that overflow that
budget force the agent to chunk, lose state, and ship lower-quality
work. Context is a first-class resource and the repo must respect it.

Caps and targets:

- **Per-file (Rust)** — hard cap 300 lines. Enforced at pre-commit
  (lefthook `file-size` hook).
- **Per-file (other source-relevant)** — soft cap 500 lines. Advisory.
- **Per-crate Rust LOC** — soft target 5_000 lines, hard cap 10_000
  lines. Enforced at pre-commit (lefthook `context-budget` hook,
  driven by `scripts/check-context-budget.sh`).
- **Per-task agent context** — informational target 10_000 lines.
  Working on a single crate or sub-area should fit that budget. If a
  task needs more, it is probably two tasks.
- **Bulk artifacts excluded** from agent default context via
  `.claudeignore` and `.cursorignore`: `Cargo.lock`, `CHANGELOG.md`,
  release-please manifests, all `*.lock` files. Agents may
  `Read` them on demand but they are not loaded by repo-orientation
  scans.

When a crate trips the soft cap, it is a signal — not a bug — that
the next refactor should consider splitting it along a real boundary
(layer, store family, sub-domain). Soft-warn is advisory; the hook
does not block. Hard-cap blocks the commit.

When `lefthook` reports `context-budget` warnings, address them in
the same PR if possible. If not, open a follow-up plan task that
names the file or crate and proposes the split.

## 14. Agent docs optimize execution over prose

Agent-facing Markdown is not marketing copy. It must be optimized for
machine execution:

- stable headings;
- short imperative rules;
- explicit file paths;
- explicit commands;
- inputs and outputs;
- acceptance criteria;
- prohibited actions;
- conflict/uncertainty behavior;
- version/status metadata when relevant.

Avoid long narrative context unless it changes an implementation
decision. If a rule cannot be verified, rewrite it until it can.

## 15. Parallel-agent work uses git worktrees, not shared checkouts

When multiple agents may operate on this repository at the same time,
each agent works in its own git worktree under
`~/convergio-worktrees/<branch>/` (or any sibling directory the user
prefers). Single-checkout `git checkout` switching is reserved for
solo human sessions.

Why: a shared checkout means every `git checkout`, `git stash`,
`git rebase`, or `git restore` is a global side effect that another
agent's tooling can read mid-flight. The classic chain of mess is one
agent rebasing branch A while another agent's `cargo build` reads a
half-applied tree on the same disk. Worktrees give each agent its own
working directory tied to its own branch, with one shared `.git`
under the hood — no checkout-races, no ambiguous "current branch".

How:

```bash
# Worktrees go under .claude/worktrees/<branch-name>/, which is
# gitignored and excluded by the cross-vendor agent ignore files
# (.claudeignore / .cursorignore / .github/copilot-ignore).
# This keeps them off git status, off agent context windows, and out
# of editor search.
git worktree add .claude/worktrees/<branch-name> -b <branch-name>
cd .claude/worktrees/<branch-name>
# work here, commit, push as usual
cd <main-checkout>
git worktree remove .claude/worktrees/<branch-name>
```

Solo human sessions are exempt. CI and automation are exempt
(GitHub Actions runs in its own runner). The rule applies when more
than one agent (`Claude`, `Codex`, `Copilot`, MCP-driven runner, or
shell scripts) might be touching the repo concurrently.

Existing branches without a worktree are grandfathered. A future plan
task may add a `cvg worktree` helper to script the setup; until then,
the two-line `git worktree add` is the canonical incantation.

## 16. Legibility score: measure that an agent can still follow the repo

Convergio v2 grew to ~100k lines in a single repository. By the time
the team noticed, no AI agent could fit the codebase in one context
window; bugs that had been hiding in the noise erupted only after
the codebase was painfully split into separate repos. This is the
exact failure mode Convergio v3 is designed to avoid.

§ 16 makes the reflex measurable. `scripts/legibility-audit.sh`
emits a score 0-100 combining four signals:

| signal | weight | what it measures |
|--------|--------|------------------|
| Cap headroom | 50 / 100 | per-file 300 cap (hard, lefthook), per-crate 5_000 soft / 10_000 hard (§ 13). Penalises near-cap files (within 50 lines) gently and crates over the soft cap meaningfully. |
| Index density | 30 / 100 | every crate under `crates/` has `AGENTS.md` (§ 11) and `CLAUDE.md`; every ADR has an explicit `Status:` line. |
| Audit-driven outcome | 20 / 100 | if a daemon is reachable, `audit/verify` returns `ok=true`. Chain corruption zeros this signal. |
| Fresh-eyes simulation | (future) | tracked as plan task T4.06 — zero-shot agent comprehension test, scored against ground-truth Q/A. |

Floor: **70 / 100**. Target: **85 / 100**. The CI step `legibility
audit` is **advisory only** — it surfaces `::warning::` annotations
on the PR but never fails the build. The gate is the static
per-file / per-crate cap; legibility is the regression-tracking
signal above it.

When the score drops, address the cause in the same PR if possible.
If not, open a follow-up plan task that names the breach (file path
or crate) and proposes the fix (split, AGENTS.md backfill, ADR
status update, audit-chain investigation).

## 17. The Modulor: every composable unit decomposes into the same shape

Convergio's atomic unit of work — its *Modulor*, in the urbanism
language of `docs/vision.md` § 4 and ADR-0018 — is the tuple

> **`(task, evidence, gate, audit_row)`**

Every operation in Convergio reduces to manipulations of this tuple.
A skill is N tasks. A wave is M tasks that ship together. A plan is
a DAG of tasks. A vertical accelerator (`convergio-edu`,
`convergio-research`, …) is a plan template plus capability blocks
plus domain gates. A city is the population of accelerators built on
the same municipality.

The Modulor is the rule that keeps the city composable. It is *not*
a metaphor — it is the literal data shape:

| Field | Storage | Why it matters |
|---|---|---|
| `task` | `tasks` table (ADR-0001) | atomic unit of agreed-upon work |
| `evidence` | `evidence` table | what the agent claims it did, in machine-readable form |
| `gate` | `crates/convergio-durability/src/gates/*.rs` (ADR-0004) | refuses with HTTP 409 if a non-negotiable is violated |
| `audit_row` | `audit_log` table, hash-chained (ADR-0002) | tamper-evident memory of every state change |

A new feature that *cannot* be expressed as a manipulation of this
tuple is, by construction, outside the urban code. Either the urban
code can absorb it (and an ADR documents how), or the feature
belongs in a capability bundle (ADR-0008) or a downstream
accelerator, not in this repo.

This rule is structural: it does not prescribe what the gate must
refuse, it prescribes that *every* refusal must produce the four
artefacts together. A behaviour that sets a value without producing
an `audit_row`, or a transition that runs without a `gate`, is a
constitutional defect, not a missing feature.

## 18. Agent merge authority

The repo is single-operator (Roberdan). Standing authorisation is
hereby granted to AI agents to **merge their own PRs without an
explicit per-PR confirmation**, provided **every** condition below
is met. The agent is acting under delegated authority; the operator
remains accountable.

### Pre-merge conditions (all required)

| # | Condition | How to verify |
|---|---|---|
| 1 | All required CI checks are `SUCCESS` | `gh pr view <N> --json statusCheckRollup` — every check `conclusion: SUCCESS` |
| 2 | The PR follows the 5-section template (Problem / Why / What changed / Validation / Impact) and includes the `## Files touched` manifest | manual read; the agent itself authored it |
| 3 | All review comments are resolved | `gh pr view <N> --json reviewDecision,comments` — no unresolved `CHANGES_REQUESTED` reviews; every comment thread either acknowledged or addressed |
| 4 | The PR title is conventional-commit shaped with a known scope | `commitlint` runs in CI as part of condition 1 |
| 5 | `mergeStateStatus == CLEAN` | `gh pr view <N> --json mergeStateStatus` |
| 6 | The PR is **not** marked draft | `gh pr view <N> --json isDraft` |

### Forbidden under any condition

- No force-push to `main` (irreversible).
- No squash or rebase merge on `main` (project policy is merge-commit only — preserves history for parallel-agent recovery).
- No bypass of branch protection (`enforce_admins=false` is a temporary convenience, not a licence).
- No skipping CI hooks (`--no-verify`, `--no-gpg-sign`).
- No merge of a PR that introduces breaking changes without an ADR.

### When in doubt — ask

If any condition is ambiguous, the agent **must** pause and ask the
operator before merging. The cost of a wasted minute is far below
the cost of an unwanted merge to `main`.

### Audit

Every agent-driven merge produces a normal merge commit on `main`
visible to `git log`. Combined with the hash-chained audit log
(ADR-0002), the operator can reconstruct who merged what, when, and
under which CI verdict, retroactively. This is the safety net that
makes the standing authorisation acceptable.

### How to revoke

The operator revokes standing authorisation by editing this section
or by direct instruction (`do not auto-merge`, `wait for me`,
`pause merges`). Revocation takes immediate effect and survives the
session.
