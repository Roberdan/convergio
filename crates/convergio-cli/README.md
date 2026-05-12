# convergio-cli (`cvg`)

Pure HTTP client for the local Convergio daemon.

```bash
cargo run -p convergio-cli -- health
cargo run -p convergio-cli -- setup
cargo run -p convergio-cli -- doctor
cargo run -p convergio-cli -- plan create "my plan"
cargo run -p convergio-cli -- plan list
cargo run -p convergio-cli -- task list <plan_id>
cargo run -p convergio-cli -- evidence add <task_id> --kind code --payload '{"diff":"fn main() {}"}'
cargo run -p convergio-cli -- solve "write docs"
cargo run -p convergio-cli -- dispatch
cargo run -p convergio-cli -- validate <plan_id>
cargo run -p convergio-cli -- demo
```

The CLI does not import `convergio-server` or any internal HTTP
route module. All inputs and outputs go through HTTP. The one
exception is `convergio-durability`: the CLI re-uses the
`Task` / `TaskStatus` / `audit::*` model types as a shared wire
schema (see `commands/agent_spawn_wire.rs`, `commands/audit.rs`,
`commands/monitor.rs`). Those types are owned by the durability
layer, are stable across the HTTP boundary, and are explicitly
out-of-scope for the "no server crates" rule in
`crates/convergio-cli/AGENTS.md`.

## Configuration

| Variable / flag | Default | Notes |
|-----------------|---------|-------|
| `CONVERGIO_URL` / `--url` | `http://127.0.0.1:8420` | daemon base URL |
| `CONVERGIO_LANG` / `--lang` | detected from environment, fallback `en` | `en` and `it` ship today |

`cvg setup` creates `~/.convergio/config.toml` and local adapter
directories. `cvg doctor` checks the local daemon, binaries, audit chain
and setup state; use `cvg doctor --json` from scripts or agents.
