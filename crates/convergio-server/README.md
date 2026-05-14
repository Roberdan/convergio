# convergio-server

Local HTTP daemon for Convergio.

The server crate is a routing shell. Business logic lives in
`convergio-durability`, `convergio-bus`, `convergio-lifecycle` and the
small Layer 4 reference crates.

## Run

```bash
cargo run -p convergio-server -- start
# SQLite at $HOME/.convergio/v3/state.db, listens on 127.0.0.1:8420
```

Installed binary:

```bash
convergio start
convergio --help
```

Configuration:

| Variable / flag | Default | Notes |
|-----------------|---------|-------|
| `CONVERGIO_DB` / `--db` | `sqlite://$HOME/.convergio/v3/state.db?mode=rwc` | SQLite URL |
| `CONVERGIO_BIND` / `--bind` | `127.0.0.1:8420` | keep localhost for local safety |
| `CONVERGIO_ALLOW_NON_LOCAL_BIND` / `--allow-non-local-bind` | `false` | explicit opt-in for non-local bind |
| `CONVERGIO_LOG` | `info` | tracing filter |

## API surface

The table below is a **quickstart subset** of the mounted routes. For
the full, always-current surface use the generated action catalog —
`GET /v1/api/actions` or `cvg actions list --output json` — and the
gate-precondition catalog at `GET /v1/gates/preconditions`. The root
`AGENTS.md` § *MCP tools available* lists every endpoint family
including status, context, CRDT, workspace, graph, embed, fleet,
telemetry, actions and gates.

| Method | Path | What it does | Layer |
|--------|------|--------------|-------|
| `GET` | `/v1/health` | liveness + version | shell |
| `GET` | `/v1/status` | dashboard summary (active plans, recent done) | shell |
| `GET` | `/v1/api/actions` | generated action catalog (full surface) | shell |
| `GET` | `/v1/gates/preconditions` | gate evidence requirements | 1 |
| `POST` | `/v1/plans` | create a plan | 1 |
| `GET` | `/v1/plans` | list plans (`?limit=`) | 1 |
| `GET` | `/v1/plans/:id` | get one plan | 1 |
| `POST` | `/v1/plans/:plan_id/tasks` | create a task | 1 |
| `GET` | `/v1/plans/:plan_id/tasks` | list tasks of a plan | 1 |
| `GET` | `/v1/tasks/:id` | get one task | 1 |
| `POST` | `/v1/tasks/:id/transition` | runs gate pipeline | 1 |
| `POST` | `/v1/tasks/:id/heartbeat` | touch task heartbeat | 1 |
| `POST` | `/v1/tasks/:id/evidence` | attach evidence | 1 |
| `GET` | `/v1/tasks/:id/evidence` | list evidence | 1 |
| `GET` | `/v1/audit/verify` | recompute hash chain | 1 |
| `POST` | `/v1/plans/:plan_id/messages` | publish message | 2 |
| `GET` | `/v1/plans/:plan_id/messages` | poll messages | 2 |
| `POST` | `/v1/messages/:id/ack` | consumer ack | 2 |
| `POST` | `/v1/agents/spawn` | spawn tracked local process | 3 |
| `POST` | `/v1/agents/spawn-runner` | spawn a vendor-CLI runner against a task (ADR-0032) | 3 |
| `GET` | `/v1/agents/:id` | get process row | 3 |
| `POST` | `/v1/agents/:id/heartbeat` | touch process heartbeat | 3 |
| `POST` | `/v1/solve` | create plan from mission | 4 |
| `POST` | `/v1/dispatch` | run one executor tick | 4 |
| `POST` | `/v1/plans/:id/validate` | run Thor validator | 4 |

Errors are JSON: `{ "error": { "code", "message" } }`.
