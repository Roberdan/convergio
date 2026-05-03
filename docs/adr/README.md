# Architecture Decision Records

We document load-bearing decisions in [MADR](https://adr.github.io/madr/)
format. Numbering is monotonic — never reuse a number.

## Workflow

1. Copy `0000-template.md` to `NNNN-short-title.md` (next free number).
2. Fill in Context, Drivers, Options, Decision.
3. Status starts at `proposed`. PR review flips it to `accepted` or `rejected`.
4. If a later decision overrides this one, set status to `superseded by NNNN`.

## Index

The table below is rewritten by `cvg docs regenerate` (ADR-0015) —
do not edit between the markers.

<!-- BEGIN AUTO:adr_index -->
| # | Title | Status |
|---|-------|--------|
| [0001](./0001-four-layer-architecture.md) | 0001. Adopt a four-layer architecture (durability, bus, lifecycle, reference) | accepted |
| [0002](./0002-audit-hash-chain.md) | 0002. Hash-chain the audit log for tamper-evidence | accepted |
| [0003](./0003-migration-coexistence.md) | 0003. Per-crate migrations on a shared `_sqlx_migrations` table | accepted |
| [0004](./0004-three-sacred-principles.md) | 0004. Three sacred principles: zero tolerance, security first, accessibility first | accepted |
| [0005](./0005-internationalization-first.md) | 0005. Internationalization first (P5) — Italian + English from day one | accepted |
| [0006](./0006-crdt-storage.md) | 0006. Model state with row and column CRDT metadata from day zero | proposed |
| [0007](./0007-workspace-coordination.md) | 0007. Coordinate multi-agent workspace changes with leases and patch proposals | proposed |
| [0008](./0008-downloadable-capabilities.md) | 0008. Install new behavior as signed isolated capabilities | proposed |
| [0009](./0009-agent-client-protocol-adapter.md) | 0009. Treat Agent Client Protocol as a future northbound editor adapter | proposed |
| [0010](./0010-retire-convergio-worktree-crate.md) | 0010. Retire the convergio-worktree crate | accepted |
| [0011](./0011-thor-only-done.md) | 0011. Done is set only by Thor (the validator) | accepted |
| [0012](./0012-ooda-aware-validation.md) | 0012. OODA-aware validation: outcome reliability over output reliability | accepted |
| [0013](./0013-split-durability-into-three-crates.md) | 0013. Split convergio-durability along three seams | proposed |
| [0014](./0014-code-graph-tier3-retrieval.md) | 0014. Code-graph layer for Tier-3 context retrieval | accepted |
| [0015](./0015-documentation-as-derived-state.md) | 0015. Documentation is derived state, not free text | accepted |
| [0016](./0016-long-tail-vertical-accelerators.md) | 0016. Convergio is the shovel for the long tail of vertical AI accelerators | proposed |
| [0017](./0017-ise-hve-alignment.md) | 0017. Convergio aligns with ISE Engineering Fundamentals + hve-core as the runtime enforcer | proposed |
| [0018](./0018-urbanism-over-architecture.md) | 0018. Urbanism over architecture: Convergio is an urban code, not a master plan | proposed |
| [0019](./0019-thinking-stack-gstack-vendored.md) | 0019. gstack ships as the Convergio thinking-stack capability | proposed |
| [0020](./0020-model-evaluation-framework.md) | 0020. Model evaluation framework — the municipality's procurement office | proposed |
| [0021](./0021-okr-on-plans.md) | 0021. Plans are Objectives + Key Results — strategic programming for the municipality | proposed |
| [0022](./0022-adversarial-review-service.md) | 0022. Adversarial review as a municipal ombudsman service | proposed |
| [0023](./0023-observability-tier.md) | 0023. Observability tier — telemetry, structured logging, request correlation | proposed |
| [0024](./0024-bus-poll-exclude-sender.md) | 0024. Bus poll filter: exclude_sender | proposed |
| [0025](./0025-system-session-events-topic.md) | 0025. The agent message bus accepts a `system.*` topic family with `plan_id IS NULL` | accepted |
| [0026](./0026-plan-wave-milestone-vocabulary.md) | 0026. Plan / wave / milestone — one vocabulary, one source of truth | accepted |
| [0027](./0027-executor-loop-wired-in-daemon.md) | 0027. Wire the Layer 4 executor loop in the daemon | accepted |
| [0028](./0028-runner-kinds-shell-claude-copilot.md) | 0028. `spawn_runner` accepts `shell`, `claude`, and `copilot` kinds | accepted |
| [0029](./0029-tui-dashboard-crate-separation.md) | 0029. TUI dashboard lives in its own crate (`convergio-tui`) | accepted |
| [0030](./0030-crate-versioning-policy.md) | 0030. Use one product version plus per-crate impact tracking | accepted |
| [0031](./0031-materialised-timing-cache.md) | 0031. Materialised timing cache + plan↔PR link table | accepted |
| [0032](./0032-vendor-cli-runners.md) | 0032. Vendor-CLI runners (no raw API calls) | accepted |
| [0033](./0033-runner-permission-profiles.md) | 0033. Vendor-CLI runners use least-privilege permission profiles | accepted |
| [0034](./0034-per-task-runner-fields.md) | 0034. Per-task runner selection (kind / profile / budget) | accepted |
| [0035](./0035-runner-registry-toml.md) | 0035. Runner registry — TOML-driven custom vendors | accepted |
| [0036](./0036-opus-backed-planner.md) | 0036. Opus-backed planner replaces the line-split heuristic | accepted |
| [0037](./0037-brand-kit-and-claim.md) | 0037. Brand kit, claim, and shared `convergio-brand` crate | accepted |
<!-- END AUTO -->
