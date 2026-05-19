# Incident — 2026-05-19 — `cvg service start` reported success without starting the daemon

**Author:** discovered while validating the `cvg service stop` fix
(PR #384). Same root cause class, found within the hour.

**Status:** fix shipped in this commit.

**Severity:** dev-loop annoyance — `cvg service start` returned
"Service started." but the daemon stayed down until the operator
ran `launchctl kickstart` manually.

## TL;DR

`cvg service start` only called `launchctl bootstrap`. After the
2026-05-08 plist hardening (`RunAtLoad=false` + `KeepAlive=false`),
`bootstrap` registers the service spec but does **not** spawn a
process — that is exactly what `RunAtLoad=false` means. The CLI
reported success because launchd accepted the registration, then
went silent. The next `curl /v1/health` failed with connection
refused.

Symmetric to the `cvg service stop` bug (incident
[2026-05-19-cvg-service-stop-orphan]): pre-hardening, `KeepAlive=true`
papered over the issue because launchd respawned the process
aggressively; post-hardening the CLI had to learn that the manager
will not move a finger without a kick.

## Why we did not catch this earlier

Same reason as the `stop` bug: the path is exercised only during a
fresh redeploy, and the kickstart-by-the-operator habit (born from
the same hardening incident) masked the lie.

## Fix

`cvg service start` (PR landing this commit) now does:

1. Install the service file (idempotent).
2. **Launchd**: `launchctl bootstrap` — ignored on failure
   ("already bootstrapped" is the common case). **Systemd**:
   `daemon-reload` + `enable --now` (which actually starts the
   service, so phases 3+4 are no-ops on linux).
3. If the daemon port is already bound (e.g. operator already had
   `convergio start` running, or a previous bootstrap is still
   serving), return success immediately.
4. On launchd, run `launchctl kickstart gui/$UID/com.convergio.v3`
   to actually spawn the process.
5. Poll the daemon port for up to 5 s. Return `Ok` when bound,
   return `Err` with a clear message if the port never comes up.

The port-polling helper `wait_for_port_bound` is the inverse of the
existing `wait_for_port_release` from the stop fix; both live in
`crates/convergio-cli/src/commands/service_port.rs`.

## Verification

```bash
# Pre-fix: daemon stays down despite "Service started."
$ cvg service stop          # actually kills it (post-PR #384)
$ cvg service start
Service started.
$ curl http://127.0.0.1:8420/v1/health   # connection refused

# Post-fix: same setup.
$ cvg service stop
$ cvg service start
Service started.
$ curl http://127.0.0.1:8420/v1/health
{"ok":true,"running_version":"0.3.29",…}
```

## Follow-up

`cvg service stop` + `cvg service start` are now both honest about
side effects. The plist hardening stays. If a Windows service
manager lands later, the same `wait_for_port_*` helpers apply.
