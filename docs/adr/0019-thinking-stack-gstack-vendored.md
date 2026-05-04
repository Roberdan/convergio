---
id: 0019
status: proposed
date: 2026-05-01
topics: [vision, capability-bundles, integration, thinking-stack]
related_adrs: [0008, 0016, 0017, 0018]
touches_crates: [convergio-mcp, convergio-cli]
last_validated: 2026-05-01
---

# 0019. gstack ships as the Convergio thinking-stack capability

- Status: proposed
- Date: 2026-05-01
- Deciders: Roberdan
- Tags: vision, capability-bundles, integration

## Context and Problem Statement

`docs/vision.md` § 6 declares three cooperating layers: gstack thinks,
hve-core/ISE engineer, Convergio governs and executes. ADR-0017
documents the alignment with Microsoft engineering work. This ADR
addresses the third corner: **how does gstack actually become
available inside the Convergio user's workflow?**

[gstack](https://github.com/garrytan/gstack) is a community
collection of Markdown-based slash-commands ("skills") for
Claude Code, Cursor, and Codex CLI. It is MIT-licensed, well
maintained, and intentionally minimalist by design: pure
Markdown, no daemon, no SQLite, no backend.

Convergio and gstack target different users. Gstack's user runs
slash-commands inside Claude Code or Cursor for ad-hoc planning
and review tasks. Convergio's user runs a daemon that enforces
gates, audits transitions, and coordinates parallel agents
through a structured plan/task lifecycle. The technology
stacks reflect this divergence: gstack is Bun/TypeScript with
Markdown content, Convergio is Rust/SQLite with HTTP+MCP. The
runtime contracts are incompatible — there is nothing wrong with
either; they solve different problems.

Convergio users still benefit from gstack's planning skills.
`/plan-ceo-review`, `/plan-eng-review`, `/plan-design-review`,
`/plan-devex-review`, `/autoplan`, `/codex` (consult/review/
challenge), `/office-hours` are genuinely useful upstream of any
Convergio plan creation. The question is *how* to make them
available inside the Convergio daemon's MCP surface without
modifying gstack and without making the operator install,
configure, and upgrade gstack as a separate moving part.

This ADR is not a critique of gstack. It is a packaging choice
for Convergio: how do we offer gstack-style planning to a
Convergio user as one less thing to set up, while leaving
gstack itself untouched and the upstream license respected.

## Decision Drivers

- **Capability registry was built for this** (ADR-0008). gstack
  becomes a capability bundle. Installation, signing, versioning,
  isolation are already first-party features.
- **gstack is MIT licensed**, so vendoring is permissible without
  coordination. Attribution is required and provided.
- **gstack updates fast** (every few days). The capability bundle
  needs an upgrade path that is safe (signed) and frequent
  (pull-based).
- **Five sacred principles must still apply** to skill output.
  gstack output is plain Markdown; some of it is in English
  hardcoded (P5 i18n violation), some has TODO comments
  (P1 zero-debt violation), some assumes screen-reader-hostile
  formatting (P3 a11y violation). Vendoring must allow Convergio
  to *overlay* its principles on gstack output without modifying
  gstack itself.
- **Honesty**: Convergio uses gstack today, informally. This ADR
  formalises what is already happening.

## Considered Options

### Option A — Don't integrate; document gstack as adjacent

README mentions "we recommend trying gstack alongside Convergio".
Costs: every user has to install, configure, and update gstack
manually. The "thinking layer" of VISION.md § 6 is aspirational,
not operational.

### Option B — Hard fork gstack into convergio

Copy `garrytan/gstack` into `crates/convergio-thinking/skills/`
as a full source-of-truth fork. Costs: maintenance burden grows
linearly with gstack's velocity (currently ~one release per 3
days). License compliance is fine but social cost is high
(forking a healthy project sends the wrong signal). Diverges
within months.

### Option C — Submodule (`git submodule`)

Add gstack as a submodule under `vendor/gstack/`. Skills are
loaded from there. Costs: submodules are operationally fragile
(`git clone --recurse-submodules` etc.); upgrade path is
manual; no signing. Bypasses ADR-0008.

### Option D — Capability bundle pulled by `cvg capability sync` (chosen)

Treat gstack as a first-party capability bundle named
`thinking-stack-gstack`, distributed via the capability registry
(ADR-0008), updated via a new `cvg capability sync` subcommand
that pulls from `garrytan/gstack` and republishes as a signed
bundle. Skills are wrapped by Convergio MCP actions
(`thinking.plan_ceo`, `thinking.plan_eng`, etc.) so the
sacred-principle overlay can apply at the wrapper layer.

## Decision Outcome

Chosen option: **Option D**, because it reuses ADR-0008
infrastructure, isolates gstack's filesystem from Convergio's,
provides a signed and versioned upgrade path, and lets Convergio
overlay its principles without forking.

### Implementation status (2026-05-04)

Stays `proposed`. No `convergio-thinking-bundles/gstack` repo is
published, no `thinking-stack-gstack` capability bundle is produced or
signed, and `cvg capability sync` does not exist. The ADR depends on
ADR-0008 graduating beyond the local install-file primitive. Until the
remote bundle pipeline ships, this ADR documents the intended
packaging choice; gstack remains an *adjacent* tool, not a Convergio
capability.

### How it works (operational sketch)

1. **Bundle source**: `convergio-thinking-bundles/gstack/`
   (separate repo, owned by Convergio org). Periodically pulls
   from `garrytan/gstack` upstream (rebase or copy, depending on
   licence terms; MIT permits both). Adds:
   - `manifest.toml` with namespace `thinking.gstack`,
     `version = "1.21.1+convergio.0"` (semver build metadata
     identifies the Convergio repackaging revision)
   - Convergio overlay: shim wrappers that intercept skill output,
     emit it through the i18n layer if needed, scrub P1-violating
     phrases, attach evidence-ready frames
2. **Signing**: every bundle revision is Ed25519-signed by
   Convergio's first-party key (per ADR-0008).
3. **Distribution**: published to the local capability registry
   as install-file. (Remote registry is deferred per ADR-0008;
   for now, install-file is the only path. ROADMAP Wave 2-3
   ships remote registry.)
4. **Installation**:
   ```
   cvg capability install-file thinking-stack-gstack-1.21.1+c0.cap
   ```
5. **Invocation via MCP**: skills become MCP actions:
   ```
   convergio.act { "type": "thinking.plan_ceo",
                   "args": { "prompt": "..." } }
   ```
6. **Upgrade**:
   ```
   cvg capability sync thinking-stack-gstack
   # → fetches latest signed bundle, verifies signature,
   #   atomically swaps installation, audit row recorded
   ```

### What the overlay does

When a gstack skill is invoked through Convergio:

- Output passes through `convergio-i18n` for any user-facing
  template strings; if the user's locale is `it-IT`, output is
  reshaped via Fluent bundles. This is best-effort (gstack output
  contains LLM-generated free text) but template literals in the
  skill itself are translated.
- Output is scanned for P1/P2 violations *as advisory* (not
  refusal) — skill output is intermediate, not evidence — and
  flagged in the audit row with `thinking.warning` events.
- Output is sanitised for screen-reader compatibility (no
  ANSI-only emphasis, no figure-without-alt-text in the rendered
  Markdown).
- Citations to gstack origin are appended (`source:
  garrytan/gstack v1.21.1, MIT licensed`).

### What this decision does not do

- It does not modify gstack's code. We re-publish, we do not
  rewrite. Upstream changes flow in.
- It does not modify upstream gstack or expect coordination with
  the gstack maintainers. MIT permits redistribution with
  attribution; we provide attribution.
- It does not block users from installing gstack directly.
  Convergio's capability is *additional*, not exclusive. A user
  can have both `~/.gstack/` (vanilla) and Convergio's wrapped
  copy without conflict.
- **Courtesy notice**: on first publication of the
  `convergio-thinking-bundles/gstack` repo, this ADR commits to
  opening a courtesy issue at upstream gstack explaining what we
  package, linking back here, and offering to remove if the
  upstream maintainer requests. This is an obligation of this
  ADR, not optional mitigation.

### What this decision *does* do

- Makes gstack's planning skills available as first-class
  Convergio MCP actions.
- Provides a documented, signed, auditable upgrade path that
  scales to gstack's release velocity.
- Lets capability bundle authors compose `thinking.gstack`
  alongside their domain capabilities (`azure.voice`,
  `auth.entra`, etc.) without writing custom integration code.
- Materialises `docs/vision.md` § 6's "thinking layer" claim as
  shippable code instead of marketing copy.

## Consequences

### Positive

- The thinking layer becomes operational. A user can run
  `/plan-ceo-review` *inside Convergio's plan-creation flow*
  without leaving the daemon's context.
- Sacred principles (especially P5 i18n) are enforced on the
  thinking layer's output without forking gstack — a clean
  separation of concerns.
- Capability bundle ergonomics are tested by a high-velocity
  upstream (gstack ships every few days). If the
  `cvg capability sync` UX survives this, it survives anything.

### Negative

- We carry the operational cost of pulling, repackaging, and
  re-signing every gstack release we want to ship. Mitigation:
  automate via GitHub Actions in `convergio-thinking-bundles`;
  release cadence does not need to match gstack 1:1.
- Drift risk: if gstack changes its skill format or filesystem
  layout, the overlay breaks. Mitigation: pin upstream version
  in the manifest; require explicit sync to pull a new gstack
  release; pre-merge tests verify the wrapper still works.
- Social risk: even with MIT redistribution being legal,
  re-publishing a community project should not be done silently.
  Mitigation is the courtesy-notice obligation above (open an
  issue upstream on first publication, offer to remove if
  requested). If upstream prefers we not publish, we stop and
  this ADR moves to status `superseded`.

### Neutral

- This ADR depends on ADR-0008 first-party bundle status. If
  ADR-0008 graduates more slowly than anticipated, this work
  also delays.

## Validation

This ADR is validated when:

1. `cvg capability install-file thinking-stack-gstack-*.cap`
   succeeds end-to-end.
2. `convergio.act { "type": "thinking.plan_ceo", … }` returns
   the same output a user would get from `claude /plan-ceo` in
   gstack v1.21.1 (modulo the i18n + a11y overlay).
3. `cvg capability sync thinking-stack-gstack` upgrades the
   installed version and writes a `thinking.upgraded` audit row
   chained to the previous version.
4. A new ADR is opened if gstack changes its skill format in a
   way that breaks the overlay; this ADR remains the policy
   anchor for "how thinking-stack lives inside Convergio".
