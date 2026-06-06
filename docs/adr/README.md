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
| [0006](./0006-crdt-storage.md) | 0006. Model state with row and column CRDT metadata from day zero | accepted |
| [0007](./0007-workspace-coordination.md) | 0007. Coordinate multi-agent workspace changes with leases and patch proposals | accepted |
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
| [0024](./0024-bus-poll-exclude-sender.md) | 0024. Bus poll filter: exclude_sender | accepted |
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
| [0038](./0038-fleet-retrieval-cross-repo-graph.md) | 0038. Fleet retrieval & cross-repo graph (semantic + multi-language) | accepted |
| [0039](./0039-doc-coherence-sweep.md) | 0039. Doc-coherence sweep as a recurring three-layer plan | accepted |
| [0040](./0040-split-coherence-into-its-own-crate.md) | 0040. Split the coherence verifiers into their own crate | accepted |
| [0041](./0041-split-session-into-its-own-crate.md) | 0041. Split the session lifecycle suite into its own crate | accepted |
| [0042](./0042-wave-sequence-gate-parallel-safe.md) | 0042. Wave-sequence gate refactor — opt-in parallel waves via per-task `parallel_safe` | accepted |
| [0043](./0043-api-id-and-payload-consistency.md) | 0043. API consistency — `id` and `payload` field naming | accepted |
| [0044](./0044-plan-execution-contract.md) | 0044. Plan execution contract — required mechanism utilization per task | accepted |
| [0045](./0045-per-host-realtime-context-push.md) | 0045. Per-host real-time context push: Cursor / Copilot / Cline strategies | accepted |
| [0046](./0046-stdout-relay-to-bus.md) | -0046 — Sub-agent stdout relay to the plan bus | accepted |
| [0047](./0047-action-type-registry-actions-json.md) | 0047. Generate a discoverable action type registry (actions.json) | proposed |
| [0048](./0048-compensating-action-types.md) | 0048. Add compensating action types | proposed |
| [0049](./0049-f3-fleet-retrospective.md) | 0049. F3 fleet-grade orchestration — retrospective | accepted |
| [0050](./0050-prompt-injection-gate.md) | 0050. PromptInjectionGate (P2 phase 1) | accepted |
| [0051](./0051-a11y-gate-phase-1.md) | 0051. A11yGate phase 1 — built-in accessibility checks on evidence | accepted |
| [0051](./0051-a11y-gate-phase-1.md) | 0051. Ontology Runtime Core (`convergio-ontology` crate) | proposed |
| [0052](./0052-smart-thor-real-validation.md) | -0052 — Smart Thor: real validation, audited pipeline runs, and skip-when-trusted | accepted |
| [0052](./0052-smart-thor-real-validation.md) | 0052. Typed Actions Framework over the Ontology | proposed |
| [0053](./0053-bitemporal-store-lineage.md) | 0053. Bitemporal Store + Lineage over Ontology Objects | proposed |
| [0053](./0053-bitemporal-store-lineage.md) | 0051. Ontology Runtime Core (`convergio-ontology` crate) | proposed |
| [0053](./0053-bitemporal-store-lineage.md) | 0053 — OpenAI vendor runner adapter | accepted |
| [0054](./0054-provenance-bundle-purpose-registry.md) | 0054. Provenance Bundle & Purpose Registry | proposed |
| [0054](./0054-provenance-bundle-purpose-registry.md) | 0054 — `cvg status --agents` live agent listing | accepted |
| [0055](./0055-entity-resolution-service.md) | 0055. Entity Resolution Service with Explainability | proposed |
| [0055](./0055-entity-resolution-service.md) | -0055 — Plan objectives table and PlanCoherenceGate | accepted |
| [0056](./0056-scenario-branching-workshop.md) | 0056. Scenario Branching (Workshop primitives) | proposed |
| [0056](./0056-scenario-branching-workshop.md) | -0056 — Parametric plan templates and `cvg plan-templates` | accepted |
| [0057](./0057-connector-sdk-federated-query.md) | 0057. Connector SDK + Federated Query | proposed |
| [0058](./0058-llm-gateway-primitive.md) | 0058. LLM Gateway primitive (typed, ontology-aware) | proposed |
| [0059](./0059-tui-ontology-inspector.md) | 0059. TUI Ontology Inspector (read-only) | proposed |
| [0060](./0060-deterministic-graph-output.md) | 0060. Deterministic Diff / Mermaid / Graphviz output format | proposed |
| [0061](./0061-capability-search-local.md) | -0061 — `cvg capability search` (local-only slice of W9) | accepted |
| [0062](./0062-dispatch-choice-audit-row.md) | -0062 — Dispatch-choice audit row (W8 slice) | accepted |
| [0063](./0063-task-taxonomy-eval-skeleton.md) | -0063 — Task taxonomy + eval outcome ledger skeleton (W10 slice) | accepted |
| [0064](./0064-a11y-axe-local-stub.md) | -0064 — A11yGate phase 2 local-stub via external `axe` binary | accepted |
| [0072](./0072-remote-capability-registry.md) | 0072. Remote capability registry (W9) | proposed |
| [0073](./0073-eu-sovereign-pivot.md) | -0073 — EU-sovereign pivot: open ontology platform, AI-native, local-first | accepted |
| [0074](./0074-relicense-agplv3.md) | -0074 — Relicense Convergio to AGPL-3.0-or-later | accepted |
| [0075](./0075-w3c-prov-provenance-bundles.md) | -0075 — W3C-PROV-JSON provenance bundles | accepted |
| [0076](./0076-gdpr-data-subject-rights.md) | -0076 — GDPR data-subject-rights handlers | accepted |
| [0077](./0077-workbench-ui-stack-tauri-vs-nextjs.md) | 0064. Choose the Convergio Workbench UI stack (Tauri-first) | proposed |
| [0078](./0078-postgres-backend-for-deployed-scale.md) | -0078 — Add a PostgreSQL backend for deployed multi-user scale | proposed |
| [0079](./0079-azure-single-tenant-deployment.md) | -0079 — Deploy verticals as customer-owned single-tenant on Azure EU | proposed |
| [0080](./0080-llm-assisted-ontology-authoring.md) | -0080 — LLM-assisted ontology authoring (ontology-author) | proposed |
| [0081](./0081-identity-and-access-for-deployed-verticals.md) | -0081 — Identity & access for deployed verticals (Entra ID OIDC + RBAC/ABAC) | proposed |
<!-- END AUTO -->
