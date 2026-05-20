<p align="center">
  <img src="assets/branding/lockup-hex-wordmark.png" alt="Convergio" width="520">
</p>

<p align="center"><strong>Convergio — Make machines prove it.</strong><br>
<em>The machine that builds machines — and proves they work.</em></p>

<p align="center">
  <img src="assets/branding/demo-gate-refusal.gif" alt="cvg demo — a dirty task is refused with HTTP 409 gate_refused, then a clean task passes Thor and the audit chain is verified" width="780">
</p>

# Convergio

[![CI](https://github.com/Roberdan/convergio/actions/workflows/ci.yml/badge.svg)](https://github.com/Roberdan/convergio/actions/workflows/ci.yml)
[![Release](https://github.com/Roberdan/convergio/actions/workflows/release.yml/badge.svg)](https://github.com/Roberdan/convergio/actions/workflows/release.yml)
[![License: Convergio Community](https://img.shields.io/badge/license-Convergio%20Community-blue)](https://github.com/Roberdan/convergio/blob/main/LICENSE)
[![Rust](https://img.shields.io/badge/rust-stable-orange)](https://www.rust-lang.org/)
[![Zero Warnings](https://img.shields.io/badge/warnings-0-brightgreen)](#)

> **Make machines prove it.** A local Rust daemon that refuses an AI
> coding agent's "done" when the evidence doesn't match the claim.

## The 30-second pitch

You let an AI agent code for you. It says **"done"**. You check —
TODOs, `unwrap()`, skipped tests, hardcoded strings. You redo it.

Convergio fixes that loop:

- 🛑 **Refuses unsafe `done`** — server-side gates reject the
  transition with HTTP 409 when evidence contains debt markers,
  scaffolding tells, secrets, or a non-clean build.
- 📒 **Tamper-evident audit** — every refusal lands in a
  hash-chained log you can verify from outside the daemon.
- 🔌 **Vendor-agnostic** — drives Claude Code, Copilot CLI, Qwen,
  Codex, Gemini, or your own runner. No API keys, no SaaS.

100% local. SQLite-only. Try it in 60 seconds → [Quickstart](#quickstart).

Convergio cannot make an agent truthful — but it raises the cost of
lying, and makes every refusal non-falsifiable.

## What Convergio is — and is not

| Convergio is… | Convergio is not… |
|---|---|
| The **execution layer** — plan → dispatch → audit → gate. | The strategic / planning layer. |
| The **leash** that decides which task runs on which model. | A model. Convergio never calls a raw API; it spawns the operator's own vendor CLI. |
| **Local-first**, single-user, SQLite-only. | A SaaS. There is no Convergio cloud. |
| **Vendor-agnostic** — claude, copilot, qwen, codex, gemini. | Tied to a single vendor. |
| **Composable** — drives any framework that emits a plan. | An agent framework. |

> Composes with planners like [`gstack`](https://github.com/garrytan/gstack)
> (strategic layer) and aligns technically with the
> [ISE Engineering Fundamentals Playbook](https://microsoft.github.io/code-with-engineering-playbook/ISE/).
> See [Composability and alignment](#composability-and-alignment) below for the full picture.

## Why Convergio

The hard failure mode for coding agents is not one bad completion. It is
many agents working at once:

- overwriting the same files;
- diverging across worktrees;
- producing broken merges and noisy CI;
- losing process-local state;
- claiming "done" without evidence.

Convergio's design answer is:

1. durable task/evidence state;
2. hash-chained audit;
3. CRDT-aware multi-actor metadata;
4. workspace resource leases;
5. patch proposals and merge arbitration;
6. server-side gates that refuse unsafe `submitted`/`done` transitions.

All six are implemented in the local runtime. The current release line
also includes signed local capability install/remove, a `planner.solve`
capability action, a constrained local shell runner proof, `shell`,
`claude`, and `copilot` runner kinds, the MCP bridge, the code graph,
the executor loop, and the `cvg dash` terminal dashboard.

Since v0.3.29 the **fleet abstraction** (ADR-0038, F3 closed in
[ADR-0049](./docs/adr/0049-f3-fleet-retrospective.md)) lets one
daemon orchestrate multiple repos. The advisory-only surface today:
cross-repo plans with per-repo task fan-out
(`POST /v1/fleet/plans` + `cvg fleet plan create`), Thor-driven
fleet validation (`POST /v1/fleet/plans/:id/validate`), derived
fleet audit-chain walk (`cvg audit verify --fleet <id>`), semantic
dead-code candidates with role-aware thresholds
(`GET /v1/fleet/rot`), and semantic doc-vs-code drift via snapshot
embeddings (`GET /v1/fleet/doc-drift` +
`POST /v1/fleet/doc-drift/snapshot`). MCP fleet actions
(`fleet_plan_create`, `fleet_plan_show`, `fleet_plan_validate`)
expose the same operations over stdio. CLI verbs for `fleet rot` /
`fleet doc-drift` are deferred until the `convergio-cli` split.
Remote capability registry and ACP bridge remain roadmap work.

See [docs/vision.md](./docs/vision.md) for the product vision.

## Principles, and which ones are actually enforced today

The five principles below are the product's identity. Each one carries
an explicit status — `enforced`, `partial`, or `planned` — so the
README does not claim more than the code does.

1. **P1 — Zero tolerance for technical debt, errors and warnings.**
   `enforced`. `NoDebtGate` (7 languages), `NoStubGate`, and
   `ZeroWarningsGate` refuse `submitted`/`done` transitions when
   evidence contains debt markers, scaffolding tells, or non-clean
   build/lint/test signals.
2. **P2 — Security first, local first.** `partial`. Localhost-by-default
   bind, evidence-as-untrusted-input, and `NoSecretsGate` (gitleaks
   pattern set) are shipped. `DepsAuditGate`, `PromptInjectionGate`,
   and HMAC middleware for non-loopback bind remain roadmap.
3. **P3 — Accessibility first.** `planned`. No `A11yGate` yet. CLI
   strives to remain screen-reader friendly without color, but this
   is convention rather than enforcement until the gate ships.
4. **P4 — No scaffolding only.** `enforced`. Two gates ship in
   `default_pipeline()`: `NoStubGate` refuses evidence that says it is
   a stub, placeholder, skeleton, or not wired; `WireCheckGate`
   (opt-in via a structured `wire_check` evidence row) verifies that
   every claimed HTTP route and CLI subcommand actually exists in the
   workspace tree. A future `ClaimCheckGate` (F55-B) will force the
   opt-in `wire_check` row to be present.
5. **P5 — Internationalization first.** `enforced`. CLI user-facing
   strings go through Fluent bundles with English and Italian
   shipped together; a coverage test refuses partial locales.

See [CONSTITUTION.md](./CONSTITUTION.md) for the full rule set, and
[docs/plans/v0.1.x-friction-log.md](./docs/plans/v0.1.x-friction-log.md)
for the gaps the next release will close.

## Quickstart

```bash
sh scripts/install-local.sh
cvg setup

convergio start
```

In another terminal:

```bash
cvg doctor
cvg health
cvg demo
```

Optional daemon service:

```bash
cvg service install
cvg service start
```

For agents:

```bash
cvg setup agent claude   # or cursor, cline, continue, qwen, shell, copilot-local
cvg mcp tail             # inspect bridge diagnostics
```

Agent hosts that support MCP should connect the stdio command
`convergio-mcp`; it exposes only `convergio.help` and `convergio.act`.
See `docs/agents/README.md` for host-specific setup snippets.

Release artifacts can be built locally with `scripts/package-local.sh`
and signed/notarized on macOS with `scripts/sign-macos-local.sh`; see
`docs/release.md`.

Defaults:

- SQLite database: `~/.convergio/v3/state.db`
- HTTP bind: `127.0.0.1:8420`
- No external services
- No account, tenant, or server setup

You can override the local database file when needed:

```bash
convergio start --db sqlite:///tmp/convergio.db?mode=rwc
```

## Manual local loop

```bash
cvg plan create "ship one clean task" --project convergio-local
cvg status
cvg task list <plan_id>
cvg task transition <task_id> in-progress --agent-id local-agent
cvg evidence add <task_id> --kind code --payload '{"diff":"fn main() {}"}' --exit-code 0
cvg evidence add <task_id> --kind test --payload '{"warnings_count":0,"errors_count":0,"failures":[]}' --exit-code 0
cvg task transition <task_id> submitted --agent-id local-agent
cvg validate <plan_id>
cvg audit verify
```

Use `cvg demo` first: it creates one dirty task that gets refused by the
gates, then one clean plan that validates and verifies the audit chain.

## What you get

Convergio is organised like a city: a stable civil code in the lower
layers, public infrastructure in the middle, and replaceable services at
the edge. Each crate has one civic role, one owner boundary, and one
reason to exist.

| Crate | Civic role | What it does | Platform value | Principle alignment |
|-------|------------|--------------|----------------|---------------------|
| `convergio-db` | Land registry | Opens the SQLite pool and runs per-crate migrations. | Gives every layer one local, durable substrate. | P2 local-first, P1 predictable schema. |
| `convergio-durability` | City hall + code enforcement | Owns plans, tasks, evidence, audit chain, gates, CRDT/workspace coordination, reaper and capability registry. | Makes work durable, reviewable and rejectable instead of conversational. | P1 gates, P2 audit/security, P4 no scaffolding, P5 evidence for localized surfaces. |
| `convergio-bus` | Postal service | Persists plan-scoped topic/direct messages and acknowledgements. | Lets agents coordinate without hidden process memory. | P2 durable local communication, P4 fully wired collaboration. |
| `convergio-lifecycle` | Workforce registry | Spawns agents, records heartbeats, watches process exit state. | Keeps agent work attached to real processes and recoverable leases. | P2 controlled local execution, P1 observable failures. |
| `convergio-graph` | Planning office map | Builds a code graph and produces task-scoped context packs. | Gives agents narrower, evidence-backed context instead of whole-repo flooding. | P1 less drift, P2 reduced prompt-injection surface, P4 wired context. |
| `convergio-server` | Public counter | Exposes the localhost HTTP API and wires the daemon loops. | One stable front door for CLI, MCP and custom runners. | P2 localhost by default, P4 routes cannot bypass gates. |
| `convergio-cli` | Clerk desk | Provides the `cvg` human/admin client over HTTP. | Lets operators inspect, drive and repair the local runtime. | P3 accessible terminal UX, P5 localized output where surfaced. |
| `convergio-tui` | Control room | Powers `cvg dash`, a read-only four-pane terminal dashboard. | Makes plans, tasks, agents and PRs visible without changing daemon state. | P3 terminal-first visibility, P2 read-only observer boundary. |
| `convergio-i18n` | Translation office | Loads Fluent bundles and tests locale coverage. | Keeps user-facing CLI strings English + Italian from day one. | P5 enforced. |
| `convergio-api` | Common legal language | Defines the stable agent action schema and schema version. | Keeps MCP/agent clients synchronized with daemon capabilities. | P1 contract tests, P4 no drift between protocol and implementation. |
| `convergio-mcp` | Embassy | Bridges MCP hosts to `convergio.help` and `convergio.act`. | Lets external agents use Convergio without learning every HTTP route. | P2 constrained tool surface, P4 action dispatch sync. |
| `convergio-planner` | Urban planner | Turns a mission into a structured plan. | Provides a reference planning flow on top of the core runtime. | P4 plans become executable tasks, not prose only. |
| `convergio-executor` | Dispatcher | Claims ready tasks and spawns configured agents; also runs as a daemon loop. | Moves ready work forward without manual polling while keeping `cvg dispatch` as a test seam. | P1 observable dispatch, P2 supervised execution, P4 fully wired Layer 4. |
| `convergio-thor` | Inspector | Validates submitted tasks and is the only component allowed to mark work `done`. | Separates "agent says done" from "system accepts done". | P1/P2/P4 final gatekeeping. |

Layer 4 is intentionally replaceable. The product value is the local
runtime and its gates; your own agent client can call the HTTP API
directly or use the MCP bridge.

```mermaid
flowchart TB
    subgraph L4["Layer 4 - replaceable city services"]
        CLI["convergio-cli\ncvg clerk desk"]
        TUI["convergio-tui\ncvg dash control room"]
        PLANNER["convergio-planner\nurban planner"]
        EXECUTOR["convergio-executor\ndispatcher loop"]
        THOR["convergio-thor\ninspector"]
        API["convergio-api\ncommon legal language"]
        MCP["convergio-mcp\nagent embassy"]
    end

    subgraph SHELL["Daemon shell"]
        SERVER["convergio-server\nlocalhost public counter"]
    end

    subgraph L3["Layer 3 - workforce registry"]
        LIFE["convergio-lifecycle\nspawn heartbeat watcher"]
    end

    subgraph L2["Layer 2 - postal service"]
        BUS["convergio-bus\nmessages ack cursor"]
    end

    subgraph L1["Layer 1 - city hall and planning map"]
        DUR["convergio-durability\nplans tasks evidence gates audit"]
        GRAPH["convergio-graph\ncode graph context packs"]
    end

    DB["convergio-db\nSQLite land registry"]

    CLI --> SERVER
    TUI --> SERVER
    MCP --> API
    MCP --> SERVER
    PLANNER --> SERVER
    EXECUTOR --> DUR
    EXECUTOR --> LIFE
    THOR --> DUR
    SERVER --> DUR
    SERVER --> BUS
    SERVER --> LIFE
    SERVER --> GRAPH
    DUR --> DB
    BUS --> DB
    LIFE --> DB
    GRAPH --> DB

    classDef law fill:#eef6ff,stroke:#2f6fed,color:#111;
    classDef service fill:#f7f7f7,stroke:#888,color:#111;
    class DUR,GRAPH law;
    class CLI,TUI,PLANNER,EXECUTOR,THOR,API,MCP service;
```

## Project status

**Current scope - local-first SQLite runtime.**

Current scope:

- SQLite-only local runtime
- localhost HTTP API
- `cvg status` snapshot dashboard for active plans and recently completed work
- `cvg dash` interactive TUI (4-pane htop-style: plans, active tasks, agents, PRs — ADR-0029)
- hash-chained audit verification
- server-side quality gates
- common local secret-leak refusal
- persistent local message bus
- task context packets and plan-scoped bus actions for MCP agents
- CRDT actor/op schema, deterministic import/merge and conflict surfacing
- workspace leases, patch proposals and merge queue arbitration
- process spawn/heartbeat/watcher
- vendor-CLI runners — `claude`, `copilot` built-in (ADR-0028,
  ADR-0032); `qwen`, `codex`, `gemini` and others via
  `~/.convergio/runners.toml` registry, no recompile (ADR-0035)
- per-task `runner_kind` / `profile` / `max_budget_usd` (ADR-0034)
- least-privilege permission profiles `standard` / `read_only` /
  `sandbox` (ADR-0033)
- opus-backed planner (`claude:opus` in `--permission-mode plan`)
  with deterministic line-split fallback (ADR-0036)
- local capability registry, Ed25519 package signature verification, and
  signed local `install-file`/remove
- `planner.solve` as the first installed capability-gated action
- executor tick, Thor validator, guided demo
- English/Italian CLI messages for the localized surfaces

Out of scope for this MVP:

- remote multi-user deployment
- account, tenant, or RBAC model
- hosted graphical UI
- hosted service
- agent marketplace

The workspace test suite covers the local runtime, gates, audit tamper
detection, CLI smoke behavior, CRDT/workspace flows, MCP actions, and HTTP
E2E workflows.

## Documentation

- [ARCHITECTURE.md](./ARCHITECTURE.md) - layers, API and request lifecycle
- [CONSTITUTION.md](./CONSTITUTION.md) - non-negotiable rules
- [docs/vision.md](./docs/vision.md) - product vision and positioning
- [docs/multi-agent-operating-model.md](./docs/multi-agent-operating-model.md) - how multiple agents coordinate through one daemon
- [ROADMAP.md](./ROADMAP.md) - focused local-first roadmap
- [CONTRIBUTING.md](./CONTRIBUTING.md) - development workflow
- [docs/adr/](./docs/adr/) - architecture decision records

## Composability and alignment

Convergio is the *machine that builds machines, safely*. A typical
full stack looks like:

| Layer | Tool | What it owns |
|---|---|---|
| **Strategic** | [`gstack`](https://github.com/garrytan/gstack) — *preferred partner* | `/autoplan`, `/office-hours`, `/plan-eng-review`, design + eng review. |
| **Memory** | gbrain (PGLite + Supabase, opt-in) | Long-term context, lessons, vault. |
| **Execution** | **Convergio** | Per-task runner routing, audit, gates, lifecycle, MCP / HTTP / CLI surface. |

You can drive Convergio with gstack's `/autoplan` (recommended),
with another planner, or by writing the plan yourself — it works
with anything that produces a mission. **gstack is the preferred
partner**: same author philosophy, same engineering-fundamentals
framing, complementary scopes. Convergio works without it; the
two are designed to compose. See
[ADR-0019](./docs/adr/0019-thinking-stack-gstack-vendored.md) for the
courtesy-notice obligations and the optional thinking-stack capability
bundle.

**Where Convergio sits in the engineering-fundamentals landscape:**
the [ISE Engineering Fundamentals Playbook](https://microsoft.github.io/code-with-engineering-playbook/ISE/)
prescribes engineering practices in checklists; community projects like
[`microsoft/hve-core`](https://github.com/microsoft/hve-core) transmit
such practices to Copilot agents via prompts and skills. Convergio is
the runtime enforcer of the principles ISE Engineering Fundamentals
describes in checklists and hve-core transmits via Copilot prompts:
gates that refuse with HTTP 409, an audit chain that proves the
refusal, an OODA loop that lets agent and validator converge or
escalate. See [ADR-0017](./docs/adr/0017-ise-hve-alignment.md) for the
mapping.

**Why we exist:** see [`docs/vision.md`](./docs/vision.md) for the
long-tail thesis and the urbanism frame (Convergio is an urban code,
not a master plan — Le Corbusier modularity + Jane Jacobs emergence).
See [`ROADMAP.md`](./ROADMAP.md) for the four waves that materialise it.

## Notices and credits

**Convergio is a personal open-source project.** It is not a Microsoft
product, not affiliated with Microsoft, and not endorsed by Microsoft.
References to the
[ISE Engineering Fundamentals Playbook](https://microsoft.github.io/code-with-engineering-playbook/ISE/)
(CC BY 4.0) and to [`microsoft/hve-core`](https://github.com/microsoft/hve-core)
(MIT) are use of those projects under their public licences and reflect
the author's reading of public documentation, not any internal position
of any organisation. References to
[`garrytan/gstack`](https://github.com/garrytan/gstack) (MIT) are
likewise public-licence use, with a courtesy-notice obligation
documented in [ADR-0019](./docs/adr/0019-thinking-stack-gstack-vendored.md).

## License

Convergio Community License v1.3 (source-available, not OSI-approved). See [LICENSE](./LICENSE).
