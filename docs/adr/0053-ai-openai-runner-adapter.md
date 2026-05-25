---
status: accepted
date: 2026-05-25
deciders: roberdan
---

# 0053 — OpenAI vendor runner adapter

## Context

W7 of the production-ready plan introduces multi-vendor agent support.
Convergio already ships runners for Claude Code, Copilot CLI, Cursor,
Cline, Continue and Qwen (see `convergio-runner/src/runner/`). We now
need an OpenAI-compatible runner so plans dispatched by the executor
can target a GPT-class model through any locally-installed
OpenAI-compatible CLI binary.

## Decision

Add a thin `OpenaiRunner` adapter to `convergio-runner` that produces a
`PreparedCommand` for an external `openai-cli`-shaped binary.

Hard constraints (ADR-0032):

- **No raw HTTP** to OpenAI from inside the daemon or any in-tree
  crate. The runner only assembles argv for a local binary the
  operator already trusts.
- The binary path is configurable via the `OPENAI_CLI_BIN` environment
  variable; default `openai-cli`. Discovery is the operator's job
  (just like `claude`, `gh copilot`, `cursor-agent`).
- The runner is pure (no `spawn`, no FS, no network). It mirrors the
  existing pattern: receives a `PreparedCommand` shape and returns
  argv + env. Tests live in
  `crates/convergio-runner/tests/runner_argv.rs` so `runner/mod.rs`
  stays under the 300-LOC cap.

Argv shape:

```
$OPENAI_CLI_BIN -p <prompt> --model <family-suffix> \
  [--permission-mode <claude-shape>] [--max-budget-usd <n>]
```

`--permission-mode` is forwarded using the same vocabulary as the
Claude runner (`bypassPermissions`, `acceptEdits`, `default`). Sandbox
runs skip the flag entirely (no escalation prompts in CI). This keeps
operators from having to learn a second permission vocabulary just
because the model vendor differs.

CLI surface:

- `cvg setup agent openai` provisions `~/.convergio/agents/openai/`
  with a `prompt.txt` whose Step 0 makes the agent register as
  `openai-${USER}-${PID}` before doing any work.
- `RunnerKind::openai_gpt()` produces the canonical
  `Family::Openai`+`gpt-4.1` kind used by the executor.

## Consequences

Positive:

- Operators can dispatch tasks to GPT-class models without Convergio
  shipping a vendor HTTP client (compliance with ADR-0032, no API key
  ever lives in the daemon process).
- Symmetric with W3 / W4 / W8: the same `PreparedCommand`+audit chain
  story works for every vendor.
- Future OpenAI-compatible binaries (Azure OpenAI CLI, vLLM CLI,
  Ollama-shimmed openai-cli) are drop-in via `OPENAI_CLI_BIN`.

Negative / trade-offs:

- We inherit whatever the local `openai-cli` does or fails to do
  (rate-limits, format drift). The runner does not retry or rewrite.
- Permission semantics are best-effort: most third-party OpenAI CLIs
  do not understand `--permission-mode`. Operators who care must wrap
  the binary themselves.

## References

- ADR-0032 (no raw vendor HTTP)
- ADR-0045 (per-host realtime context push)
- ADR-0051, ADR-0052 (W1/W3 sibling workstreams)
- `crates/convergio-runner/src/runner/openai.rs`
- `crates/convergio-runner/tests/runner_argv.rs`
