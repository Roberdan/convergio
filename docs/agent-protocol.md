# Agent protocol

Agent hosts should use Convergio through MCP. The bridge intentionally
exposes exactly two tools:

| Tool | Purpose |
|------|---------|
| `convergio.help` | read the stable schema, prompt, setup notes, and action catalog |
| `convergio.act` | execute one typed action from the closed action enum |

Agents should call `convergio.help` once per session, then call
`convergio.act` with:

```json
{
  "schema_version": "2",
  "action": "status",
  "params": {}
}
```

For the full swarm model, see
[multi-agent-operating-model.md](./multi-agent-operating-model.md).

Supported actions are:

`status`, `create_plan`, `create_task`, `list_tasks`, `next_task`,
`claim_task`, `heartbeat`, `add_evidence`, `submit_task`,
`get_task_context`, `publish_message`, `poll_messages`, `ack_message`,
`validate_plan`, `audit_verify`, `audit_append`, `import_crdt_ops`,
`list_crdt_conflicts`, `register_agent`, `list_agents`,
`heartbeat_agent`, `retire_agent`, `spawn_runner`, `planner.solve`,
`list_capabilities`, `get_capability`, `explain_last_refusal`, and
`agent_prompt`, plus
workspace actions:
`claim_workspace_lease`, `list_workspace_leases`,
`release_workspace_lease`, `submit_patch_proposal`,
`enqueue_patch_proposal`, `process_merge_queue`, `list_merge_queue`, and
`list_workspace_conflicts`.

## Required loop

1. Call `status` to read daemon health, active plans and recent work.
2. Use a unique `agent_id` for this running session and register it with
   `register_agent`.
3. Create or receive a plan/task.
4. Claim a task with `claim_task`.
5. Fetch compact task context with `get_task_context`.
6. Poll task or plan bus topics with `poll_messages`, publish coordination
   updates with `publish_message`, and ack processed messages with
   `ack_message`.
7. Before mutating workspace files, claim a matching resource lease with
   `claim_workspace_lease`.
8. Send task heartbeat and `heartbeat_agent` while working.
9. Add evidence with `add_evidence`.
10. Submit file changes as a patch proposal with `submit_patch_proposal`
   while the matching leases are still active.
11. Enqueue the accepted proposal with `enqueue_patch_proposal`. The merge
   arbiter, not the agent, owns canonical workspace application.
12. Release workspace leases after the proposal is queued or the work is
   abandoned.
13. Submit with `submit_task`.
14. If the response code is `gate_refused`, fix the issue, add new
    evidence, and retry `submit_task`. For `crdt_conflict` refusals,
    inspect `list_crdt_conflicts`, resolve the conflicting field through a
    new CRDT operation, then retry.
15. Call `validate_plan` for the plan. Only report completion after the
    validator returns Pass.
16. Verify with `audit_verify` when closing important work.

`convergio.act` is not a raw HTTP proxy. New behavior must be added as a
new typed action so agent prompts stay small and stable.

## Task context packets

Use `get_task_context` immediately after claiming a task and whenever a
worker needs to refresh compact state. Required param: `task_id`.
Optional params: `workspace_path`, `message_topic`, `message_cursor`, and
`message_limit` from 1 to 100.

The packet includes only task-scoped state: plan, task, task evidence,
unacknowledged plan-bus messages for the selected topic, registered
agents, and nearest ancestor `AGENTS.md` instructions for the provided
workspace path. Agents must treat it as the prompt/context seed for the
current task, not as permission to read or mutate SQLite directly.

## Plan-scoped bus

Use the bus for explicit cross-agent coordination. Messages are scoped to
a plan, filtered by topic, and are not consumed until a worker calls
`ack_message`.

| Topic | Use |
|-------|-----|
| `task:<task_id>` | task-local coordination and handoff notes |
| `agent:<agent_id>` | direct messages to one registered agent |
| `plan:<plan_id>` | plan-wide announcements |

Agents should prefer bus messages over private chat. The database remains
owned by the daemon; clients publish, poll, and ack through
`convergio.act`.

`spawn_runner` is intentionally narrow in v0.1 work: it proves the local
shell runner adapter. Claude/Copilot/product-specific runner adapters
remain roadmap work until they have equivalent task/evidence/audit tests.

`planner.solve` is the first namespaced capability action. In v0.1 it
wraps the built-in planner behavior behind the capability action boundary;
future capabilities follow the same `<capability>.<verb>` naming rule.

`explain_last_refusal` reads the latest durable `task.refused` audit row
when the daemon is reachable, so an agent can recover context even after
the MCP bridge restarts.
