# Multi-agent operating model

This document answers the practical question: how do multiple Claude,
Copilot, Cursor, Cline, shell, or custom agents use one Convergio without
creating chaos?

## Short version

Agents do not coordinate by chatting directly. They coordinate by using
the same local Convergio daemon.

```text
Claude Code  ─┐
Copilot CLI  ─┤
Cursor agent ─┼──> convergio-mcp / HTTP / cvg ──> Convergio daemon
Cline        ─┤                                  └─> SQLite + audit + gates
shell agent  ─┘
```

Convergio is the shared state, lock manager, message bus, evidence
store, gatekeeper, and future merge arbiter. Agents are workers.

## Two valid ways to use it

### Mode 1: human-opened swarm

The user opens multiple agent sessions manually:

```text
Terminal 1: Claude Code
Terminal 2: Copilot CLI
Terminal 3: Cursor agent
Terminal 4: shell runner
```

Each host is configured with the same MCP bridge:

```bash
cvg setup agent claude
cvg setup agent copilot-local
cvg setup agent cursor
```

All of them point to:

```text
http://127.0.0.1:8420
```

Each agent calls `convergio.help`, gets the same protocol, asks for work
with `next_task`, claims one task, heartbeats, adds evidence, submits,
and obeys refusals.

This is the first practical multi-agent mode.

### Mode 2: Convergio-orchestrated swarm

A lead agent or human creates a plan. Convergio decomposes or receives
tasks, then launches worker agents through registered runner adapters.

```text
user/lead agent
  -> create plan
  -> solve plan into tasks
  -> dispatch runnable tasks
  -> runner adapters spawn workers
  -> workers claim/heartbeat/evidence/submit
```

Today this is proven only for the constrained local shell runner exposed
as `spawn_runner`. Product-quality Claude/Copilot/Cursor runner adapters
are future work. Until those adapters exist, use Mode 1 for those hosts.

## What a single agent must do

### Session lifecycle is automatic

The project-level Claude Code `SessionStart` hook
(`.claude/settings.json`) runs `cvg session register-and-poll`
before the first user prompt. That single call:

1. Registers (or refreshes) the agent identity in `agent_registry`.
2. Sends an immediate heartbeat.
3. Lists active plans and polls `agent:<id>` and `plan:<id>`
   topics on each.
4. Publishes a `session-started` envelope on every active plan's
   `coordination/agents` topic so peers see the new session.

The audit kind `agent.session_started` is emitted only when the
agent is new or the previous heartbeat is older than 30 minutes —
re-running the hook on a session resume does not spam the chain.

`/v1/status.telemetry` exposes seven aggregate counters
(`agents_registered_total`, `agents_active_24h`,
`sessions_started_24h`, `plans_active`, `audit_rows_total`,
`bus_messages_24h`, `workspace_leases_active`) so `cvg dash` and
the multi-agent operator can see "no agent is registering" without
staring at audit rows.

If your harness has no `cargo` on PATH, copy
`.claude/settings.local.json.example` to
`.claude/settings.local.json` to point at the precompiled
`~/.convergio/bin/cvg` instead.

### Manual loop

Every agent session needs a unique `agent_id`, for example:

```text
claude-architect-01
copilot-impl-03
cursor-reviewer-02
```

The loop is:

1. Call `convergio.help`.
2. Call `agent_prompt` to get the current Convergio instructions.
3. Call `status`.
4. Use the active-plan dashboard to understand current work.
5. Get work with `next_task` or receive an assigned task.
6. Claim it with `claim_task`.
7. Send heartbeat while working.
8. Fetch task context with `get_task_context`.
9. Coordinate through `poll_messages`, `publish_message`, and
   `ack_message`.
10. For workspace-changing tasks, request leases for
    files/directories/symbols.
11. Work in an isolated sandbox/worktree.
12. Submit a patch proposal instead of merging directly.
13. Wait for the merge arbiter and gates.
14. Add evidence.
15. Submit.
16. If refused, read `explain_last_refusal`, fix, add new evidence, retry.
17. Only report done after Convergio accepts.

## Does the database act as context?

Yes, but not as a giant chat transcript.

The database is durable operational context:

| Context type | Stored in Convergio |
|--------------|---------------------|
| plan goal | plan record |
| task scope | task record |
| dependencies | task graph |
| agent identity | agent/session record |
| instructions | agent prompt + task description |
| progress | heartbeat + task status |
| discussion | message bus |
| facts/proof | evidence |
| refusal reasons | audit + gate output |
| future conflicts | CRDT/workspace conflict records |

Agents should not paste entire conversations into every task. Convergio
should give each worker a compact task packet:

```text
plan summary
task objective
constraints
allowed resources
relevant prior evidence/messages
required output/evidence
local folder instructions
```

That is how we avoid boiling agents with too much context.

## Should agents talk to each other?

Not directly.

Direct agent-to-agent chat is invisible, unaudited, and impossible to
replay. Agents may communicate through Convergio:

| Need | Channel |
|------|---------|
| announce progress | task status / heartbeat |
| ask another role for input | message bus topic |
| hand off findings | evidence |
| block unsafe work | lease/conflict/refusal |
| explain failure | audit/refusal record |

The message bus is the communication channel. It is persisted in SQLite,
scoped to a plan, and can be replayed. Agents can have skills/roles, but
coordination still goes through the daemon.

## Agent names, roles, and skills

Convergio needs three separate concepts:

| Concept | Example | Purpose |
|---------|---------|---------|
| `agent_id` | `claude-impl-01` | unique running worker identity |
| `actor_id` | UUID | CRDT identity for writes/imported ops |
| role/skills | `rust`, `review`, `docs` | scheduling and task matching |

Do not overload one field for all three.

## Agent registry kinds

`agent_registry.kind` is a permissive lower-case string. Validation
(see `convergio-durability::store::agent_validation`) only requires
`[a-z0-9._-]{1,64}`, so new hosts can land without a schema change.
For consistency across `cvg agent list`, the TUI dashboard, and
`cvg coherence agents`, use one of the documented kinds below
whenever you can:

| `kind` | Used by | Notes |
|--------|---------|-------|
| `claude` | top-level Claude Code session | registered by `/cvg-attach` |
| `claude-code` | alias for `claude` (legacy) | accepted; prefer `claude` |
| `copilot` | GitHub Copilot CLI session | |
| `cursor` | Cursor agent | |
| `codex` | OpenAI Codex CLI | |
| `aider` | Aider | |
| `shell` | shell-runner spawned by the executor | |
| `subagent` | Claude Code subagent (Task tool) | wrapped by `/cvg-spawn` (see § Subagent lifecycle below) |

## Subagent lifecycle

Claude Code subagents — the helpers a parent session launches via
the `Task` tool — run in a different harness from the top-level
session. The `SessionStart` hook in `.claude/settings.json` does
**not** fire for them, so without action they are invisible to
the daemon.

The `/cvg-spawn` skill closes that gap. It generates the canonical
register / heartbeat / retire wrapper and the parent agent prepends
the rendered block to the subagent brief. The contract:

1. **Register with `kind=subagent`.** The `agent_id` is a
   derivative of the parent's identity / task description, e.g.
   `subagent-${TASK_DESC_SLUG}-${HEX8}`. Using a derivative id
   makes parentage visible in `cvg agent list` even though the
   registry itself does not model edges.
2. **Heartbeat ~every 5 min** while the subagent is working. The
   reaper (default 5-minute timeout) flips silent agents to
   `unhealthy`, which surfaces in the dashboard.
3. **Retire on finish.** The subagent always POSTs to
   `/v1/agent-registry/agents/${id}/retire` before exiting,
   including on failure paths, so the registry does not collect
   zombies. Top-level sessions do this via the `Stop` hook;
   subagents do it inline at the end of their brief.
4. **TUI rendering.** `cvg dash` lists subagents alongside
   top-level sessions but in the dim-text style — they are
   support workers, not first-class swarm members.

The `/cvg-spawn` skill performs no network I/O itself; it only
emits the wrapper text. The parent agent is therefore free to
audit (or modify) the rendered block before pasting it into the
brief — Convergio still records the resulting register / retire
in the audit chain.

## Cross-agent peer-review through observability

A peer-review between two Claude Code sessions happened on 2026-05-01
without a single direct message: session B read the live plan and
audit chain, recognised gaps in session A's discipline, and filed
them as numbered findings in `docs/plans/*-friction-log.md`. Session
A reconciled the numbering and merged both batches.

The lesson is small and load-bearing:

- The audit chain is sufficient observability for one agent to
  review another's work without a chat channel.
- Markdown conventions (frontmatter, F-numbered findings) are a
  contract that survives across agents because every agent reads
  the same `AGENTS.md` and friction logs.
- Convention beats coordination protocol. The bus carries the ack;
  the review itself is observability.

For a real-time view of bus traffic on a plan, the canonical
human / agent verb is `cvg bus tail --plan <id> --follow`, which
consumes the SSE feed at `/v1/plans/:plan_id/messages/stream` and
prints each message as it lands. The TUI dashboard (`cvg dash`)
exposes the same data in a Bus pane (P1.3, future work).

Open gaps the dogfood made visible (still applicable):

- **Push notifications.** SSE shipped in P1.1 + `cvg bus tail
  --follow` in P1.2; a session that does not subscribe still misses
  the handshake. Websocket / outbound webhook remain future work.
- **Skill-aware assignment.** Both sessions knew their territories
  because a human said so. No scheduler exists.
- **File-level conflict prevention.** Workspace leases exist as an
  API surface but neither agent claimed one. Disjoint territories
  were luck, not enforcement.

## What works today

Implemented:

- one local daemon, SQLite-only;
- MCP bridge with `convergio.help` and `convergio.act`;
- plan/task/evidence lifecycle (with `task.closed_post_hoc` for triage);
- task claim, heartbeat, retry;
- gate refusals + durable `explain_last_refusal`;
- hash-chained audit;
- durable agent registry (spawn, heartbeat, retire, watcher reaper);
- task context packets;
- plan-scoped bus + `system.*` topic family (ADR-0025) with `exclude_sender` filter (ADR-0024);
- CRDT actor/op store and conflict listing;
- workspace resources, leases, patch proposals, merge queue, conflicts;
- local capability registry with Ed25519 signature verification (signed
  install-file only — no remote registry yet);
- vendor-CLI runners: shell, claude, copilot — plus TOML-driven custom
  vendors (ADR-0028, 0032, 0035) with permission profiles (ADR-0033)
  and per-task runner fields (ADR-0034);
- executor loop wired in the daemon (ADR-0027);
- planner capability action `planner.solve` (Opus-backed, ADR-0036);
- TUI dashboard `cvg dash` (ADR-0029);
- Tier-3 code-graph retrieval `cvg graph` (ADR-0014);
- real-time SSE on `/v1/audit/stream` and
  `/v1/plans/:id/messages/stream` (P1.1) — agents and dashboards
  receive events as they happen instead of poll-only access.

Not implemented yet:

- skill-aware scheduling and automatic assignment;
- remote (downloadable) capability registry;
- websocket push (the SSE channel above is what shipped);
- cross-repo embedding fleet (ADR-0038, F1 in flight).

## What must be built next

To make this feel like "open one Convergio plan and let it run a swarm",
the next core pieces are:

1. **Skill-aware scheduling** — match `next_task` to an agent's
   declared capabilities so an orchestrator no longer hand-routes work.
2. **Push notifications on the bus** — SSE or websocket so a session
   sees `agent:<id>` direct messages without polling.
3. **Remote capability registry** — capabilities downloaded locally
   only after signature verification (ADR-0008).
4. **Cross-repo retrieval fleet** — embedding-based context retrieval
   across multiple repos (ADR-0038).

## Anti-chaos rules

1. Agents never write directly to Convergio SQLite.
2. Agents never coordinate important decisions outside Convergio.
3. Agents never mark work complete without accepted Convergio state.
4. Agents never mutate the canonical workspace directly once leases and
   patch proposals exist.
5. Context is task-scoped, not repo-wide chat history.
6. Every crate/folder has local agent instructions for responsibility and
   invariants.
7. New orchestration behavior lives behind daemon APIs and tests, not
   only in prompts.

## The mental model

Convergio is not "a better prompt".

Convergio is the local control plane:

```text
planner creates tasks
workers claim tasks
leases protect resources
evidence proves work
gates refuse unsafe transitions
messages coordinate handoffs
patch proposals protect Git
merge arbiter updates canonical state
audit proves what happened
```

The agents can be Claude, Copilot, Cursor, Cline, shell scripts, or
future capabilities. The rule is the same: they work through Convergio,
not around it.
