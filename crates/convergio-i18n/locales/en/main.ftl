# Convergio — English message bundle.
# Fluent syntax: https://projectfluent.org/fluent/guide/

# ---------- generic ----------
ok = OK
not-found = Not found
internal-error = Internal error

# ---------- daemon ----------
daemon-starting = Starting Convergio daemon at { $url }
daemon-listening = Listening on { $bind }
daemon-version = Convergio { $version }

# ---------- CLI: health ----------
health-ok = Daemon is healthy. Version: { $version }
health-unreachable = Could not reach daemon at { $url }: { $reason }
health-drift = WARNING drift: workspace expects { $expected }, daemon running { $running }. Run `cvg update`.

# ---------- CLI: pre-dispatch drift warning (P1-2) ----------
cli-drift-warning = WARN: convergio CLI is v{ $cli } but daemon at { $url } is running v{ $daemon }
cli-drift-fix-hint = WARN: run `cvg service restart` (or restart the daemon manually) to pick up the latest changes.
cli-drift-suppress-hint = WARN: suppress with { $env }=1

# ---------- CLI: update ----------
update-rebuild-header = Rebuilding daemon, CLI, and MCP binaries...
update-rebuild-step = building { $crate }
update-sync-header = Syncing shadowed binaries
update-restart-header = Restarting daemon
update-restart-skipped = Skip restart (--skip-restart): daemon left as-is
update-verify-header = Verifying
update-no-update-needed = No update needed: daemon already at { $version }
update-summary-ok = cvg update done: { $prior } -> { $new } (restarted: { $restarted })
update-step-failed = step '{ $step }' failed with code { $code }
update-sync-copy-warning = Warning: could not copy { $src } to { $dst }: { $reason }
update-release-notes-header = Latest release notes:
update-release-notes-unavailable = Release notes unavailable (gh CLI missing or offline).
update-changelog-header = CHANGELOG between prior and new version:
update-changelog-empty = No CHANGELOG entries between prior and new version.
update-changelog-not-found = CHANGELOG.md not found; skipping --changelog output.

# ---------- CLI: status ----------
status-header = Convergio status
status-active-header = Active plans:
status-active-empty = No active plans.
status-completed-header = Recently completed plans:
status-completed-empty = No completed plans yet.
status-tasks-header = Recently completed tasks:
status-tasks-empty = No completed tasks yet.
status-plan-line = - { $title } [{ $status }] project: { $project } tasks: { $done }/{ $total } done
status-progress-line =   progress: { $bar } { $done }/{ $total }
status-breakdown-line =   tasks: { $done } done · { $submitted } submitted · { $in_progress } in-progress · { $pending } pending · { $failed } failed ({ $total } total)
status-work-line =   does: { $work }
status-next-line =   next: { $tasks }
status-wave-line =     wave { $wave }: { $done } done, { $submitted } submitted, { $in_progress } in-progress, { $pending } pending, { $failed } failed
status-mine-header = Filter: showing only tasks for agent { $agent }
status-task-line = - { $title } in { $plan } project: { $project }

# ---------- CLI: CRDT ----------
crdt-conflicts-empty = No unresolved CRDT conflicts.
crdt-conflicts-header = Unresolved CRDT conflicts:
crdt-conflict-line = - { $entity }/{ $id } field { $field } type { $type }

# ---------- CLI: workspace ----------
workspace-leases-empty = No active workspace leases.
workspace-leases-header = Active workspace leases:
workspace-lease-line = - { $agent } holds { $kind } { $path } until { $expires }

# ---------- CLI: capabilities ----------
capabilities-empty = No local capabilities registered.
capabilities-header = Local capabilities:
capability-line = - { $name } { $version } [{ $status }]
capability-signature-ok = Capability signature verified for { $name } { $version } with key { $key }
capability-installed = Capability installed: { $name } { $version } [{ $status }]
capability-disabled = Capability disabled: { $name } { $version }

# ---------- CLI: setup / doctor ----------
setup-config-created = Config created: { $path }
setup-config-exists = Config already exists: { $path }
setup-config-backed-up = Existing config backed up: { $path }
setup-config-repo-path-added = Backfilled missing repo_path in: { $path }
setup-complete = Setup complete: { $path }
setup-next-start = Next: start the daemon with `convergio start`
setup-next-doctor = Then: run `cvg doctor`
setup-agent-created = Adapter snippets created for { $host }: { $path }
setup-agent-copy = Copy mcp.json into the agent host MCP configuration and prompt.txt into its instructions.
setup-agent-claude-extras = Claude Code extras: copy skill-cvg-attach/ into ~/.claude/skills/cvg-attach/ and merge settings.json into ~/.claude/settings.json so SessionStart registers this session with the local daemon. See { $path }/README.txt for the full steps.
setup-self-check-header = Convergio install self-check (ADR-0044)
setup-self-check-ok = OK   { $name }: { $message }
setup-self-check-warn = WARN { $name }: { $message }
setup-self-check-fail = FAIL { $name }: { $message }
setup-self-check-summary-ok = Self-check passed.
setup-self-check-summary-fail = Self-check failed — fix FAIL items before starting a task.
doctor-header = Convergio doctor for { $url }
doctor-ok = OK { $name }: { $message }
doctor-warn = WARN { $name }: { $message }
doctor-fail = FAIL { $name }: { $message }
doctor-summary-ok = Doctor passed.
doctor-summary-fail = Doctor found failing checks.
mcp-log-missing = No MCP log found yet.
service-installed = Service file written: { $path }
service-started = Service started.
service-stopped = Service stopped.
service-status-loaded = Service is loaded.
service-status-not-loaded = Service is not loaded.
service-uninstalled = Service uninstalled.

# ---------- CLI: plan ----------
plan-created = Plan created: { $id }
plan-renamed = Plan renamed: { $id } -> { $title }
plan-transitioned = Plan { $id } moved to status: { $status }
plan-not-found = Plan not found: { $id }
plan-list-empty = No plans yet.
plan-list-header = { $count ->
    [one] One plan:
   *[other] { $count } plans:
}
plan-list-line = #{ $number } { $title } [{ $status }]

# ---------- CLI: plan run ----------
plan-run-started = Running plan #{ $number }: { $title } ({ $pending } pending tasks)
plan-run-task-submitted = [{ $wave }.{ $seq }] { $title } → submitted ✓
plan-run-halted = Halted at task [{ $wave }.{ $seq }] { $title }: { $error }
plan-run-complete = Plan #{ $number } complete: { $count } tasks submitted.
plan-run-resume-hint = Resume with: cvg plan run { $number }

# ---------- CLI: plan triage ----------
plan-triage-empty = No stale tasks found (pending/failed, not touched in { $days } days).
plan-triage-header = { $count ->
    [one] One stale task (pending/failed, not touched in { $days } days):
   *[other] { $count } stale tasks (pending/failed, not touched in { $days } days):
}
plan-triage-line = - [{ $status }] w{ $wave }.{ $seq } { $title } [{ $id }] (last update: { $updated_at })
plan-triage-confirm = Close these { $count } tasks? [y/N]:
plan-triage-closed = Closed { $count } tasks.
plan-triage-skipped = Triage cancelled — no tasks closed.

# ---------- CLI: agent ----------
agent-list-empty = No registered agents.
agent-list-header = { $count ->
    [one] One agent:
   *[other] { $count } agents:
}
agent-list-header-active = { $count ->
    [one] One active agent:
   *[other] { $count } active agents:
}
agent-list-stale-hidden = { $count ->
    [one] ({ $count } stale/terminated agent hidden — use --all to show)
   *[other] ({ $count } stale/terminated agents hidden — use --all to show)
}
agent-list-col-id = ID
agent-list-col-kind = KIND
agent-list-col-status = STATUS
agent-list-col-current-task = CURRENT TASK
agent-list-col-task = TASK
agent-list-col-branch = BRANCH
agent-list-col-last-hb = LAST_HB
agent-list-col-claimed = CLAIMED
agent-list-col-last-topic = LAST_TOPIC
agent-list-col-capabilities = CAPABILITIES
agent-list-col-leases = LEASES
agent-list-col-last-audit = LAST_AUDIT
agent-show-header = Agent { $id }:
agent-show-kind = Kind
agent-show-status = Status
agent-show-registered = registered { $at }
agent-show-capabilities = Capabilities
agent-show-last-topic = Last bus topic
agent-show-no-last-topic = no bus activity
agent-show-claimed-tasks = Claimed tasks
agent-show-no-claimed-tasks = no claimed tasks
agent-show-current-task = Current task
agent-show-no-current-task = no current task
agent-show-plan = Plan
agent-show-task-status = Status
agent-show-leases = Active workspace leases
agent-show-no-leases = no leases
agent-show-recent-audit = Recent audit
agent-show-no-recent-audit = no recent audit
agent-show-recent-prs = Recent PRs by this agent
agent-show-no-recent-prs = no recent PRs
agent-retire-stale-summary = { $count ->
    [one] Retired { $count } stale agent (threshold { $threshold_min } min):
   *[other] Retired { $count } stale agents (threshold { $threshold_min } min):
}
agent-retire-stale-dry-run = { $count ->
    [one] Would retire { $count } stale agent (dry-run, threshold { $threshold_min } min):
   *[other] Would retire { $count } stale agents (dry-run, threshold { $threshold_min } min):
}
agent-retire-stale-none = no stale agents under the threshold
agent-retire-success = Agent { $id } retired
agent-retire-not-found = Agent not found: { $id } (already retired or never registered)
agent-retire-help-after-422 = Heartbeat cannot set status='retired' — use `cvg agent retire { $id }` (or POST /v1/agent-registry/agents/{ $id }/retire).
agent-not-found = Agent not found: { $id }

# ---------- gate refusals (human side) ----------
# The `code` field stays English (it's an API contract).
# The `message` is what the human reads.
gate-refused-evidence = Missing evidence: { $kinds }
gate-refused-no-debt = Technical debt found in evidence: { $markers }
gate-refused-no-stub = Scaffolding markers found in evidence: { $markers }
gate-refused-zero-warnings = Build/lint signal is not clean: { $signals }
gate-refused-plan-status = Plan is { $status }; cannot accept new transitions
gate-refused-wave-sequence = { $count ->
    [one] One earlier-wave task is still open
   *[other] { $count } earlier-wave tasks are still open
}

# ---------- audit ----------
audit-clean = Audit chain verified: { $count } events, no tampering detected.
audit-broken = Audit chain broken at sequence { $seq }.

# ---------- CLI: pr stack ----------
pr-stack-empty = No open PRs.
pr-stack-header = { $count ->
    [one] One open PR:
   *[other] { $count } open PRs:
}
pr-stack-no-manifest = no Files-touched manifest
pr-stack-manifest-mismatch = manifest does not match diff
pr-stack-files-summary = { $count ->
    [one] one file
   *[other] { $count } files
}
pr-stack-suggested-order = Suggested merge order:

# ---------- CLI: session resume ----------
session-resume-header = Convergio session resume
session-resume-health-ok = Daemon: ok (version { $version })
session-resume-health-down = Daemon: NOT ok (version { $version })
session-resume-audit-ok = Audit chain: ok ({ $count } events)
session-resume-audit-broken = Audit chain: BROKEN ({ $count } events checked)
session-resume-plan-line = Plan: { $title } [{ $status }] project: { $project } id: { $id }
session-resume-counts-line = Tasks: { $done }/{ $total } done — in_progress: { $in_progress }, submitted: { $submitted }, pending: { $pending }
session-resume-next-empty = Next priority: none (no pending tasks).
session-resume-next-header = Next priority (top pending):
session-resume-next-line =   - w{ $wave }.{ $sequence } { $title } [{ $id }]
session-resume-prs-empty = Open PRs: none.
session-resume-prs-unavailable = Open PRs: gh not available (skipped).
session-resume-prs-header = Open PRs:
session-resume-pr-line =   - #{ $number } { $title } ({ $branch })
session-resume-pr-line-draft =   - #{ $number } [draft] { $title } ({ $branch })
session-resume-pack-line = Context-pack for task { $task_id }: { $nodes } matched nodes, { $files } files, ~{ $est_tokens } tokens

# ---------- CLI: session register-and-poll ----------
session-register-poll-header = Convergio session register-and-poll
session-register-poll-registered = Registered as: { $id } (kind={ $kind }, host={ $host })
session-register-poll-heartbeat = Heartbeat: { $status }
session-register-poll-plans-header = { $count ->
    [0] Active plans: none
    [one] Active plans (1):
   *[other] Active plans ({ $count }):
}
session-register-poll-plan-line =   - { $id } { $title }
session-register-poll-direct-header = { $count ->
    [0] Pending direct messages: none
    [one] Pending direct messages (1):
   *[other] Pending direct messages ({ $count }):
}
session-register-poll-announcements-header = { $count ->
    [0] Pending plan announcements: none
    [one] Pending plan announcements (1):
   *[other] Pending plan announcements ({ $count }):
}
session-register-poll-message-line =   - plan { $plan } seq { $seq } [{ $topic }] sender={ $sender }
session-register-poll-message-line-consumed =   - plan { $plan } seq { $seq } [{ $topic }] sender={ $sender } (consumed)

# ---------- brand (CLI: about) ----------
# Brand marks (claim/subline/product name) are NOT translated — they
# are trade dress and live in `convergio-brand`. These keys are the
# *labels* surrounding the brand mark when the CLI explains itself.
brand-about-tagline = Convergio — { $version }
brand-about-source = Source: { $url }
brand-about-help = Type `cvg --help` to get started.

# ---------- CLI: coherence routes ----------
coherence-routes-summary = Checked { $code } code routes against { $docs } documented routes; { $violations } drift item(s).
coherence-routes-ok = Routes coherence: ok (no drift).
coherence-routes-header = Routes coherence: { $count } drift item(s):
coherence-routes-missing-in-docs = missing_in_docs: { $method } { $path } (in code at { $file }, not documented)
coherence-routes-missing-in-code = missing_in_code: { $method } { $path } (documented in { $file }, not in code)
coherence-routes-method-mismatch = method_mismatch: { $path } — code has [{ $code_methods }], docs have [{ $doc_methods }]

# ---------- CLI: coherence adrs ----------
coherence-adrs-summary = Checked { $checked } ADRs, { $findings } finding(s).
coherence-adrs-empty = ADR coherence: ok (no status drift detected).
coherence-adrs-table-header = ADR    Declared                         Finding                      Evidence
coherence-adrs-finding-accepted-no-evidence = accepted, no evidence
coherence-adrs-finding-proposed-likely-shipped = proposed, likely shipped
coherence-adrs-finding-broken-supersession = broken supersession

# ---------- CLI: coherence agents ----------
coherence-agents-summary = Checked { $checked } merged PRs in [{ $since }], { $findings } finding(s); strict_passes={ $strict }.
coherence-agents-empty = Agents coherence: no merged PRs in window.
coherence-agents-table-header = PR     Author                 Agent matched            Finding                    Evidence
coherence-agents-finding-no-registered-agent = no_registered_agent
coherence-agents-finding-no-heartbeat = no_heartbeat_in_window
coherence-agents-finding-no-coordination = no_coordination
coherence-agents-finding-clean = clean

# ---------- CLI: coherence handshake (F1) ----------
coherence-handshake-summary = cvg coherence handshake — daemon: { $daemon } (timeout { $timeout }ms)
coherence-handshake-phase-1 = register A+B
coherence-handshake-phase-2 = A → ping
coherence-handshake-phase-3 = B receives + pongs
coherence-handshake-phase-4 = A receives pong
coherence-handshake-phase-5 = acks
coherence-handshake-phase-6 = retire
coherence-handshake-success = handshake complete in { $elapsed }ms (timeout was { $timeout }ms)
coherence-handshake-fail = handshake failed after { $elapsed }ms (timeout was { $timeout }ms)
coherence-handshake-timeout = handshake timed out after { $elapsed }ms (deadline { $timeout }ms)

# ---------- CLI: coherence plan-execution (ADR-0044) ----------
coherence-plan-execution-summary = Plan { $plan }… — { $closed } closed task(s), { $compliant } compliant, score { $score }%
coherence-plan-execution-plan-checks = Plan-level: registry={ $registry }  bus={ $bus }
coherence-plan-execution-task-ok = OK   { $id }… { $title }
coherence-plan-execution-task-fail = FAIL { $id }… { $title } — missing: { $missing }

# ---------- CLI: bus tail / list (P1.2) ----------
bus-tail-following = Following bus on plan { $plan } (Ctrl-C to exit)
bus-tail-disconnect = bus stream disconnected, reconnecting...
bus-tail-streaming-unavailable-fallback-polling = WARN: daemon does not advertise streaming; falling back to 1s polling.
bus-tail-empty = No messages.
bus-list-summary = Plan { $plan } — { $count } message(s)

# ---------- CLI: discover (F2) ----------
discover-header = Convergio peer discovery (as of { $at })
discover-active-peers = ACTIVE PEERS (heartbeat in last { $since }, status != terminated/retired):
discover-recent-bus = RECENT BUS ACTIVITY (top 5 topics, last 1 hour):
discover-your-plans = YOUR PLANS (where your agent_id appears, latest first):
discover-empty-peers = (no active peers in window)
discover-empty-bus = (no recent bus activity)
discover-empty-plans = (no plans assigned to you)

# ---------- CLI: task complete orchestrator (P1-1) ----------
task-complete-step-graph = [complete] graph for-task --semantic …
task-complete-step-embed = [complete] embed for-task …
task-complete-step-evidence-graph = [complete] evidence add graph_pack …
task-complete-step-evidence-embed = [complete] evidence add embed_query …
task-complete-step-evidence-pr = [complete] evidence add pr_link (PR #{ $pr }) …
task-complete-step-submit = [complete] transition → submitted …
task-complete-step-thor = [complete] validate plan (Thor) …
task-complete-thor-failed = Thor validation failed: { $verdict }
