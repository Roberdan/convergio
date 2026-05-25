---
id: 0050
status: accepted
date: 2026-05-25
topics: [security, gates, prompt-injection, p2]
related_adrs: [0004]
touches_crates: [convergio-durability]
last_validated: 2026-05-25
---

# 0050. PromptInjectionGate (P2 phase 1)

- Status: accepted
- Date: 2026-05-25
- Tags: security, gates, prompt-injection, p2

## Context

CONSTITUTION § Sacred principle #2 ("Security first, including
LLM-specific threats") was advertised in the README as `partial`:
localhost-by-default and `NoSecretsGate` shipped, but no gate
inspected evidence payloads for LLM-prompt-injection content.

The threat model that motivates this gate is **not** "the agent's
LLM gets jailbroken at runtime" (we don't proxy LLM traffic). It
is **"a hostile artefact gets pulled into the audit trail"**: a
README fetched from the web, a comment in a third-party
dependency, a transcript pasted into a `log` evidence row. The
next agent that loads this evidence as context will see those
strings and may follow them. Convergio's whole value proposition
("the leash") collapses if hostile text can ride on evidence rows
unchallenged.

The existing `NoSecretsGate` already shows the shape of the
solution: a regex pass over all string leaves of every evidence
payload at `submitted` / `done`, with stable refusal reasons
surfaced via the gate-precondition catalog (P3-2).

## Decision

Add `PromptInjectionGate` to `default_pipeline()` right after
`NoSecretsGate`, so all evidence-payload scans live next to each
other. Pattern set is closed and curated, not a single mega-regex,
so individual rules can be retired or amended without invalidating
the gate's identity.

Phase 1 ships these eight rule families:

| Rule name              | What it catches                                                   |
|------------------------|-------------------------------------------------------------------|
| `instruction_override` | "ignore (previous\|prior\|above) (instructions\|prompts\|rules)" |
| `instruction_disregard`| "disregard everything above"                                      |
| `role_override_persona`| "you are now DAN / developer mode / jailbroken / unrestricted"    |
| `system_prompt_exfil`  | "reveal/print/show/repeat … (system) prompt/instructions"         |
| `role_tag_chatml`      | ChatML `<|im_start|>system` markers spliced into payloads         |
| `markdown_script_link` | Markdown link whose target uses `javascript:` / `data:` / `vbscript:` |
| `role_confusion_line`  | Line-starting `system:` / `assistant:` / `user:` fake turns       |
| `invisible_unicode`    | Zero-width and bidi-override characters used to smuggle text      |

Refusal reason format mirrors `NoSecretsGate`:
`prompt_injection_pattern_found: <evidence_kind>#<rule>, …`. The
gate is active only on `submitted` and `done` transitions; all
intermediate transitions remain unaffected so agents can iterate
freely on `in_progress` evidence.

### Opt-out

Legitimate quotation (this gate's own tests, security documentation
that cites payload literals, training material) needs an escape
hatch. The gate honours two:

1. JSON key `"pi_gate_exempt": true` at any depth in the evidence
   payload.
2. String marker `__prompt_injection_gate_exempt__` anywhere in
   any string leaf of the payload.

Either marker short-circuits scanning for that *single* evidence
row. Other rows on the same task still get scanned. The marker is
audited as part of the row payload itself — the audit chain
remembers exactly which evidence opted out.

## Consequences

- **README P2 moves from `partial` to `enforced` for evidence-side
  prompt-injection.** Outbound LLM-traffic inspection remains out
  of scope by design (Convergio does not proxy model calls).
- Cost is negligible: 8 small regexes over string leaves of
  evidence payloads at exactly two transitions per task.
- Pattern set is intentionally closed-and-curated; rules added
  later will reuse the same name → rule → reason pipeline and do
  not require a new ADR unless the gate's semantics change.
- False-positive surface is bounded by the explicit opt-out. Any
  refusal an agent considers wrong is fixable in-payload without
  changing the gate.

## Alternatives considered

- **Single mega-regex** — rejected. Hard to extend, hard to
  attribute refusals, defeats the per-rule reason surface that
  P3-2 callers depend on.
- **External TOML pattern file** (`patterns/prompt_injection.toml`
  as the v1.0 plan suggested) — deferred. The benefit is hot-update
  without a release; the cost is one more I/O failure mode at
  daemon start. Phase 1 keeps the patterns embedded; the TOML file
  becomes worthwhile when the rule count grows past ~30.
- **Refuse on `in_progress` too** — rejected. Would block routine
  agent iteration with no real safety win; the leash bites at
  `submitted` / `done`, consistent with the rest of the pipeline.
- **Inspect downstream LLM input/output** — out of scope. Convergio
  is a state machine over evidence, not a proxy over model traffic.
