# convergio-lifecycle

Layer 3 of Convergio — agent process supervision.

## Status

**Implemented (basic).** Spawn + persist row + heartbeat + mark-exited
work end-to-end. The supervisor records spawn failures as `failed` rows,
returns a specific error for invalid persisted timestamps, and bounds the
async bookkeeping around spawn with a default timeout.

The watcher loop detects unexpected exits on POSIX platforms with
`kill -0`. Windows PID probing is intentionally unsupported in the MVP;
on Windows the watcher leaves rows as `running` until a platform-specific
probe is implemented.

## API

| Op | Function |
|----|----------|
| Spawn | `Supervisor::spawn(SpawnSpec { kind, command, args, env, plan_id, task_id, cwd, stdin_payload })` |
| Spawn with timeout | `Supervisor::spawn_with_timeout(spec, duration)` |
| List | `Supervisor::list(limit)` |
| Get | `Supervisor::get(id)` |
| Heartbeat | `Supervisor::heartbeat(id)` |
| Mark exited | `Supervisor::mark_exited(id, exit_code, ok)` |

`cwd` selects the working directory for the spawned child (inherited
when `None`). `stdin_payload` is the prompt piped on the child's
stdin and then closed — used by vendor-CLI runners (claude, copilot,
qwen) that read non-interactively. A write failure on that pipe is
surfaced as a `SpawnFailed` error and the persisted row is marked
`failed`; missing-prompt-but-API-says-success is not a valid outcome.

HTTP surface (mounted by `convergio-server`):

| Method | Path |
|--------|------|
| `POST` | `/v1/agents/spawn` |
| `GET`  | `/v1/agents/:id` |
| `POST` | `/v1/agents/:id/heartbeat` |

## Use case

Plan with a 6-hour critical task. Agent's context window dies after
2 hours. Layer 3 records the missing heartbeat against
`agent_processes` and the OS-watcher flips the row out of `running`
when the PID is gone. The actual task-level recovery — releasing the
task back to `pending` after a configurable inactivity window
(default 5 minutes, see `CONVERGIO_REAPER_TIMEOUT_SECS`) — is owned
by the Layer 1 reaper in `convergio-durability`, not by this crate.
The executor (Layer 4) then picks the task up with a new agent.

This crate intentionally does not claim a "60-second" heartbeat
deadline of its own: heartbeat timing is the durability layer's
contract, and the OS-watcher polls on its own
(`CONVERGIO_WATCHER_TICK_SECS`, default 30s) for liveness.

## What it is NOT

- **Not** systemd / launchd — we don't manage system services.
- **Not** a sandbox — agents run with the daemon's privileges.
- **Not** Kubernetes — no resource limits, no scheduling, no networking.
