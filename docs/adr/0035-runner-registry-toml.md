---
id: 0035
status: accepted
date: 2026-05-03
topics: [runners, configuration, vendors, agents]
related_adrs: [0028, 0032, 0033, 0034]
touches_crates: [convergio-runner, convergio-executor, convergio-cli]
last_validated: 2026-05-03
---

# 0035. Runner registry — TOML-driven custom vendors

- Status: accepted
- Date: 2026-05-03
- Tags: runners, configuration, vendors

## Context

ADR-0028 introduced runner kinds (`shell`, `claude`, `copilot`).
ADR-0032 narrowed Convergio to vendor CLIs only. ADR-0033 added
permission profiles. ADR-0034 made `runner_kind` per-task.

The hard-coded `Family { Claude, Copilot }` enum still blocked the
goal stated by the project owner: adding a new vendor (`qwen`,
`codex`, `gemini`) should not require a Convergio recompile. Only
the *vendor families* are expensive to add — once a CLI's flag
shape is captured, models change frequently and should be cheap.

## Decision

Introduce a runner registry at `~/.convergio/runners.toml`. Each
top-level `[vendors.<name>]` block declares a `RunnerSpec`:

```toml
[vendors.qwen]
cli           = "qwen"
prompt_via    = "stdin"          # "stdin" | "argv"
prompt_argv_flag = "-p"          # used only when prompt_via = "argv"
model_flag    = "--model"
extra_args    = ["--no-stream"]
models        = ["qwen-coder", "qwen-max"]   # optional allowlist

[vendors.qwen.profiles.standard]
args = ["--read-only=false"]

[vendors.qwen.profiles.read_only]
args = ["--read-only=true"]

[vendors.qwen.profiles.sandbox]
args = ["--allow-all"]
```

`RunnerKind.vendor` is now a free-form `String`. When the vendor
matches a built-in family (`claude`, `copilot`) the hardcoded
runner runs unchanged. Otherwise `for_kind_with_registry` resolves
the vendor through the registry and builds a `ConfigRunner` whose
argv shape comes from the spec. Empty `models` → any model is
accepted (so new models surface without a registry edit).

The default registry path is `$HOME/.convergio/runners.toml`,
overridable via `$CONVERGIO_RUNNERS_TOML` for tests. A missing
file yields an empty registry (the default install).

## Consequences

- The `Family` enum stays closed at `Claude | Copilot`; adding a
  new built-in still requires a Rust impl, but the registry path
  covers everything the user explicitly asked for.
- `ConfigRunner` uses the same `prompt::build` contract as the
  built-ins — heartbeat / evidence / transition / PR convention
  stay uniform across vendors.
- The permission-profile envelope (ADR-0033) is honored: the spec
  declares per-profile flag sets so least privilege still applies
  to custom vendors.
- The executor and `cvg agent spawn` load the registry once at
  startup; tests use `RunnerRegistry::empty()` so no `~/.convergio`
  IO sneaks into unit tests.

## Alternatives considered

- **Open the `Family` enum to `Custom(String)`.** Cleanest at the
  type level but ripples through every match site (cli + tag now
  return `&str` not `&'static str`, `RunnerError::CliMissing` loses
  its `&'static str`). Rejected: too much churn for too little
  payoff vs. a separate registry path.
- **YAML/JSON registry.** Convergio already depends on `toml` for
  workspace metadata; adding a second config format is friction.
- **Inline argv string per task.** Worst case: every task carries
  its own argv shape. Removes the planner's ability to reason
  about cost / model and defeats the leash.

## Validation

- `cargo fmt --all -- --check`
- `RUSTFLAGS="-Dwarnings" cargo clippy --workspace --all-targets -- -D warnings`
- `RUSTFLAGS="-Dwarnings" cargo test --workspace`
- `RunnerRegistry::parse` round-trips the example below.
- A task with `runner_kind = "qwen:qwen-coder"` resolves through
  the registry once `~/.convergio/runners.toml` declares `qwen`,
  and surfaces `RunnerError::UnknownVendor` otherwise.
