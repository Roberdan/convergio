# Agent host setup

All supported hosts use the same Convergio MCP bridge:

```bash
convergio-mcp --url http://127.0.0.1:8420
```

Generate the exact snippets for your host:

```bash
cvg setup agent <host>
```

Supported hosts:

| Host | Command |
|------|---------|
| Claude Desktop / Claude Code | `cvg setup agent claude` |
| GitHub Copilot local IDE integrations | `cvg setup agent copilot-local` |
| GitHub Copilot cloud agent repository hint | `cvg setup agent copilot-cloud` |
| Cursor | `cvg setup agent cursor` |
| Cline | `cvg setup agent cline` |
| Continue | `cvg setup agent continue` |
| Qwen / qwen-code | `cvg setup agent qwen` |
| Generic shell agent | `cvg setup agent shell` |

Each generated directory contains:

| File | Use |
|------|-----|
| `mcp.json` | copy into the host MCP configuration |
| `prompt.txt` | copy into custom instructions |
| `README.txt` | host-local reminder |

## Step 0 — register every session

Before doing anything else, every host session must register itself in
the local agent registry so peer sessions can see it and the daemon
gets a heartbeat to detect liveness. The bootstrap is baked into the
generated `prompt.txt` for each host (see `cvg setup agent <host>`).

The `agent_id` is host-shaped to prevent collisions when two sessions
of the same host run on the same machine:

| Host | `agent_id` placeholder |
|------|------------------------|
| `claude` (Claude Code / Desktop) | `claude-code-${USER}` |
| `copilot-local` | `copilot-local-${USER}-${PID}` |
| `copilot-cloud` | `copilot-cloud-${REPO_FULL_NAME}-${RUN_ID}` |
| `cursor` | `cursor-${USER}-${WORKSPACE}` |
| `cline` | `cline-${USER}` |
| `continue` | `continue-${USER}` |
| `qwen` | `qwen-${USER}` |
| `shell` | `shell-${USER}-${PPID}` |

Bootstrap (curl fallback when `cvg session register-and-poll` is not
yet available in the installed cvg version):

```bash
curl -fsS -X POST http://127.0.0.1:8420/v1/agent-registry/agents \
  -H 'Content-Type: application/json' \
  -d '{"id":"<your-agent-id>","kind":"<host>","name":"<descriptive>","host":"<machine>","capabilities":["..."]}'

curl -fsS -X POST http://127.0.0.1:8420/v1/agent-registry/agents/<your-agent-id>/heartbeat \
  -H 'Content-Type: application/json' -d '{"status":"idle"}'
```

If those calls fail, the daemon is down: `cvg service start`, then retry.

## Required agent behavior

1. Call `convergio.help` once.
2. Use `convergio.act`; do not call daemon HTTP endpoints directly.
3. Use a unique `agent_id` for each running session.
4. Claim tasks before working.
5. Send heartbeat while working.
6. Attach evidence before submit.
7. If `gate_refused`, fix the root cause, attach new evidence, retry.
8. Only tell the user work is complete after Convergio accepts the task.

For multi-agent usage, do not let agents coordinate through private chat
or side files. They should coordinate through Convergio task state,
evidence, audit, and the plan-scoped message bus. See
`docs/multi-agent-operating-model.md`.

## Troubleshooting

```bash
cvg doctor --json
cvg mcp tail
convergio-mcp --version
```

If `doctor` says the daemon is unreachable:

```bash
cvg service start
# or
convergio start
```
