---
id: 0045
status: accepted
date: 2026-05-05
topics: [agents, hooks, bus, hosts, context]
related_adrs: [0023, 0044]
touches_crates: [convergio-cli-session]
last_validated: 2026-05-05
---

# 0045. Per-host real-time context push: Cursor / Copilot / Cline strategies

- Status: accepted
- Date: 2026-05-05
- Deciders: Roberto D'Angelo, claude-code-roberdan (F3 of plan `db812b00`)
- Tags: agents, hooks, hosts

## Context and Problem Statement

PR #220 (P1-3) shipped a Claude Code `PreToolUse` hook that auto-fires
`cvg session heartbeat-since-last-turn` and a `SessionStart` hook
(extended in #223 / P2-6) that auto-fires `cvg session register-and-poll`
plus `cvg session resume`. The combined effect: every Claude Code
session is registered, heart-beats while it works, drains its bus
inbox before the first user prompt, and gets a live cold-start packet
for free.

That works because Claude Code exposes `SessionStart`, `PreToolUse`,
and `Stop` lifecycle hooks. The other supported hosts do not have
the same surface:

- **Cursor** — `.cursor/rules/` files are loaded once per session and
  prepended as context, but there is no per-tool-call hook.
- **GitHub Copilot CLI** — `copilot config` is config-only; the CLI
  has no documented agent lifecycle hooks today.
- **Cline (VS Code)** — `.vscode/extensions` and the Cline settings
  pane configure behavior but do not run shell commands per turn.

If we ship only the Claude Code path, peer agents under Cursor /
Copilot / Cline silently violate ADR-0023 (every session must be
visible in the registry) and ADR-0044 (every task must exercise the
mechanisms required by its evidence contract). The luck-based silence
gap from the 2026-05-04 retrospective comes back.

## Decision

For each host without a per-tool-use hook, ship the strongest
workaround the host actually supports today, document the gap
clearly, and degrade gracefully. The same `cvg session ...`
subcommands stay the source of truth across hosts.

### Host strategy matrix

| Host          | Lifecycle surface available    | Strategy                                                                                    |
| ------------- | ------------------------------ | ------------------------------------------------------------------------------------------- |
| Claude Code   | SessionStart + PreToolUse + Stop | Native hooks (P1-3, P2-6). Reference implementation. Real-time inbox drain + heartbeat.    |
| Cursor        | `.cursor/rules/` (load-once)   | Always-loaded rule injects a "before every assistant turn, run `cvg session poll-since-last-turn`" instruction. Best-effort, model-driven. |
| Copilot CLI   | none documented                | Document the manual `cvg session resume` pattern. Suggest a per-shell `precmd` wrapper if the user wants automation.                       |
| Cline         | `.vscode/settings.json` only   | `.cursor/rules/`-style instruction file (`.cline/rules.md`) + same model-driven fallback as Cursor.                                        |
| Continue      | similar to Cline               | Same rules-file fallback.                                                                   |
| Qwen / shell  | none                           | Manual; document `cvg session resume` + `cvg session poll-since-last-turn` in `prompt.txt`. |

### Reference integration

`examples/skills/cvg-attach-cursor/` ships:

- `rules.md` — Cursor rule file that registers the session, polls
  the inbox before each turn, and re-runs `cvg session resume` if
  the user types `/resume`. Copy into `.cursor/rules/cvg-attach.md`
  in any repo to opt-in.
- `README.md` — paste-ready install instructions and a description
  of the gap vs Claude Code.

Cline / Continue follow the same shape (different file path); a
single `examples/skills/cvg-attach-rules-file/` directory ships the
generic rule body with placeholders for the host name and rule path.

### What we do NOT promise

- **Cursor / Cline / Continue cannot guarantee real-time push.** They
  rely on the model honoring the rule. If the model skips it, the
  inbox drains late. This is a known limitation, called out in
  the README of each example.
- **Copilot CLI** has no automation today. The README documents the
  manual workflow and links the upstream issue we are tracking.

## Consequences

- Every host has a documented, paste-ready integration path. No host
  is silently in violation of ADR-0023.
- Claude Code stays the gold standard for real-time context push; the
  rules-file approach is best-effort.
- When Cursor / Copilot / Cline expose real lifecycle hooks, the
  reference integrations migrate without breaking the shared
  `cvg session ...` surface — the daemon API does not change.

## Validation

- `examples/skills/cvg-attach-cursor/README.md` install steps work
  end-to-end on a clean repo (manual smoke).
- `cvg setup agent <host>` for cursor / cline / continue / copilot
  emits a `prompt.txt` that references this ADR for the per-host
  caveat.

## Alternatives considered

- **Wait for upstream hooks**: rejected — we cannot block the
  multi-agent operating model on third-party roadmaps.
- **A daemon-side polling cron**: rejected — pushes work to the
  daemon side that should be host-driven, and breaks the "session
  is the actor" model from ADR-0023.
- **Stub the hosts as "unsupported"**: rejected — Cursor / Cline are
  in active use; honesty by partial coverage beats silence.
