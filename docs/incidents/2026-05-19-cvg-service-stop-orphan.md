# Incident — 2026-05-19 — `cvg service stop` did not stop the daemon

**Author:** observed by Roberto, root cause + fix landed by Claude (claude-opus-4-7) on 2026-05-19.
**Status:** root cause identified, fix shipped in this commit.
**Severity:** dev-loop annoyance — daemon redeploys silently kept running the previous build.

## TL;DR

After merging the F3-5 / F3-6 / F3-8 PRs and rebuilding with
`scripts/install-local.sh`, the running daemon stubbornly reported
the old version (`0.3.26`) even after `cvg service stop && cvg
service start`. Direct inspection showed an `~/.cargo/bin/convergio
start` PID launched on **Monday morning** (May 18) still bound to
`127.0.0.1:8420` — three `service stop` cycles had returned
"Service stopped." without actually killing it.

Root cause: the post-2026-05-08 launchd plist hardens against the
respawn loop by setting `KeepAlive=false` + `RunAtLoad=false`.
Combined with the existing `cvg service stop` implementation,
which only calls `launchctl bootout`, this means:

- `launchctl bootout` removes the **service definition** from the
  domain. It does **not** terminate processes that were launched
  outside of launchd (e.g. by a previous `convergio start` from a
  terminal, or by a launchctl bootstrap that has since been
  superseded).
- The orphan PID therefore keeps the daemon port held, and the
  next `launchctl bootstrap` does nothing because launchd does
  not look at port state — it just registers the spec.
- `cvg service start` reports `Service started.` because launchd
  said the bootstrap succeeded, even though no new process was
  actually spawned (the old one already had the port).

The next `curl /v1/health` then returns the **old version**,
because the old process is still answering. From the operator
seat this looks like `service stop` is silently broken.

## Why we did not catch this earlier

`cvg service stop` is rarely exercised — daily work talks to the
daemon over HTTP and doesn't care which PID is serving it. The
incident only surfaces during a redeploy that bumps the version,
which is exactly what happened today after merging release
0.3.29. The pre-2026-05-08 plist (`KeepAlive=true`) papered over
the issue because launchd would aggressively kill+respawn the
managed PID, masking the orphan-PID class altogether.

## Fix

`cvg service stop` (PR landing this commit) now does:

1. Ask the service manager to stop (`launchctl bootout` on macOS,
   `systemctl --user stop` on Linux). Best-effort — ignore the
   exit code, because the port-release check below is the real
   contract.
2. Poll the daemon port (default `8420`, overridable via
   `CONVERGIO_URL`) for up to 3 s. Return success as soon as
   the port is free.
3. If the port is still bound: find the holder PID (`lsof` on
   macOS, `ss` on Linux), send `SIGTERM`, wait up to 3 s.
4. Still bound? Send `SIGKILL`, wait up to 2 s.
5. Still bound? Return `Err` so the operator sees something is
   wrong (rather than the previous `Ok` lie).

`cvg service stop_best_effort` (used by `uninstall`) inherits the
same flow.

## Verification

Reproduce the original failure mode and the fix:

```bash
# Pre-fix: daemon survives 'service stop' if launched outside launchd.
$ ~/.cargo/bin/convergio start &
$ cvg service stop  # returns "Service stopped." but PID is alive
$ curl -s http://127.0.0.1:8420/v1/health  # still answers

# Post-fix: same setup.
$ ~/.cargo/bin/convergio start &
$ cvg service stop
# Either silent success (manager-only path) or:
# "warning: daemon PID 12345 survived service manager stop; sending SIGTERM"
$ curl -s http://127.0.0.1:8420/v1/health  # connection refused — correct
```

## Follow-up

- The `KeepAlive=false` / `RunAtLoad=false` plist hardening stays.
  It is the right answer to the 2026-05-08 self-killer, and the
  current fix sits at the right layer: the CLI verifies the
  outcome, the plist stays minimal.
- If we add a Windows service manager later, the same
  port-verification gate applies.
