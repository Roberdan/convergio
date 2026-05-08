# convergio-cli-session

Session lifecycle commands for Convergio — the `cvg session` suite
(ADR-0041). Extracted from `convergio-cli` so the verifiers stay
embeddable from skills, runners, and future MCP bridges without
shelling out to `cvg`.

Primary subcommands:

- `cvg session resume` — cold-start brief: daemon health, audit
  chain, active plan, next-priority pending tasks, open PRs, and
  (with `--task-id`) a Tier-3 graph context-pack.
- `cvg session pre-stop` — end-of-session safety net (PRD-001 §
  Artefact 4): walks a registry of cheap checks and refuses to
  detach when one finds something unless `--force` is passed.

Hook wiring subcommands (host SessionStart / PreToolUse):

- `cvg session register-and-poll` — register + heartbeat + poll inbox
  on every active plan.
- `cvg session heartbeat-since-last-turn` — best-effort, throttled
  heartbeat for “still alive” telemetry.

Render the API:

```bash
cargo doc --open -p convergio-cli-session
```

The `convergio-cli` crate hosts a thin shim
(`crates/convergio-cli/src/commands/session.rs`) that re-exports
`SessionCommand` and `run` so the dispatcher in `main.rs` keeps the
same call shape.
