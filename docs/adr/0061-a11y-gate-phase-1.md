---
id: 0061
status: accepted
date: 2026-05-25
topics: [accessibility, gates, durability, p3]
related_adrs: [0004, 0050]
touches_crates: [convergio-durability]
last_validated: 2026-05-25
---

# 0061. A11yGate phase 1 — built-in accessibility checks on evidence

- Status: accepted
- Date: 2026-05-25
- Tags: a11y, gates, p3, v1.0
- Workstream: W1 of `docs/plans/v1.0-production-ready.md`

## Context

CONSTITUTION § Sacred principle #3 declares accessibility-first as
non-negotiable. ADR-0004 ratified this. Until W1 the gate pipeline
had no a11y enforcement: a directly-quoted gap in README (`P3:
planned`) and in the v1.0 plan.

The plan asked for two phases:

1. **Phase 1 (this ADR)**: built-in checks that ship with the daemon
   and require no external tooling. Cheap, always-on, narrow in
   scope.
2. **Phase 2 (W11)**: installable capability `a11y.axe` wrapping
   axe-core for full WCAG-2.2 AA coverage. Out of scope here.

## Decision

Add `A11yGate` to `default_pipeline()` between `NoSecretsGate` and
`ZeroWarningsGate` (slot 8 of 11). The gate is active on
`Submitted` and `Done` transitions and refuses with HTTP 409 +
reason `a11y_violation_found: <evidence_kind>#<rule>, ...`.

### Rule set (closed list, embedded)

| Rule                       | Kinds                                       | Catches                                                  |
|----------------------------|---------------------------------------------|----------------------------------------------------------|
| `md_heading_skip`          | markdown, markdown_doc, md_doc, doc, readme | Forward jump of more than one heading level (H1 → H3+)   |
| `md_image_missing_alt`     | markdown, markdown_doc, md_doc, doc, readme | `![](url)` / `![ ](url)` or `<img ...>` with missing/blank `alt=` |
| `md_link_nondescriptive`   | markdown, markdown_doc, md_doc, doc, readme | `[here]`, `[click here]`, `[link]`, `[this]`, `[read more]`, plus `<a ...>click here</a>` |
| `md_color_only_emphasis`   | markdown, markdown_doc, md_doc, doc, readme | `<font color=...>` — emphasis carried by color only      |
| `md_color_contrast_low`    | markdown, markdown_doc, md_doc, doc, readme | Inline `style="color:#...; background-color:#..."` with contrast < 4.5:1 |
| `cli_color_only_signal`    | cli_output, terminal, tui_snapshot          | Line whose meaning vanishes when ANSI escapes are stripped |
| `bidi_override`            | **all kinds**                                | U+202A..U+202E and U+2066..U+2069 spoofing characters    |

Markdown and CLI rules are scoped to their evidence kinds. The bidi
check fires on every kind because text-direction spoofing works in
any text leaf.

### Evidence-kind dispatch

The gate inspects only the kinds it knows it can analyse. Adding a
new evidence kind that should be a11y-scanned means extending
`is_markdown_kind` or `is_cli_kind`. Unknown kinds are not silently
flagged.

## Consequences

- **README P3 moves from `planned` to `enforced (built-in checks,
  phase 1; axe-core in phase 2)`.** Phase 2 (W11) will compose with
  this gate, not replace it.
- Cost is negligible: a small set of regex scans over string leaves,
  plus a tiny contrast calculator for inline hex colors, at exactly two
  transitions per task.
- Pattern set is intentionally closed-and-curated; rules added
  later reuse the same name → rule → reason pipeline and do not
  require a new ADR unless the gate's semantics change.
- False-positive surface for the link-nondescriptive rule is bounded
  to a handful of stock phrases. Documentation that legitimately
  uses one of those phrases must rephrase — this is the desired
  behaviour, not a bug.

## Alternatives considered

- **Wait for phase 2 (axe-core capability) and ship a single
  gate** — rejected. The capability sits behind W9 (remote
  capability registry) and W11, both still ahead of v1.0. The
  built-in checks here cover the highest-frequency a11y violations
  (missing alt text, heading skips, color-only emphasis) at zero
  install cost.
- **Hard fail on every markdown without metadata** — rejected. The
  gate must refuse genuine violations, not absence of opt-in
  schemas. Adding required metadata is the planner's job.
- **String-scoped opt-out** — explicitly rejected on the same
  threat-model grounds as ADR-0050: payload text is untrusted and
  must not be able to self-bypass.

## References

- ADR-0004 — Three sacred principles
- ADR-0050 — PromptInjectionGate (same pipeline slot pattern, same
  threat-model stance on opt-outs)
- `docs/plans/v1.0-production-ready.md` § W1
