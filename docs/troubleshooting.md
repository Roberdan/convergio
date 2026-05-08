---
topic: troubleshooting
---

# Troubleshooting (common pitfalls)

This is a local-first daemon + CLI. Most issues come down to one of:

- binaries not on `PATH`
- daemon not running / wrong URL
- port already in use
- SQLite file permissions / locking
- unsigned macOS artifacts being quarantined

## 1) `cvg: command not found` / `convergio: command not found`

If you installed via `scripts/install.sh`, binaries land in `~/.local/bin`.

```bash
ls -la ~/.local/bin | grep -E '^(.* )?(convergio|cvg|convergio-mcp)$' || true
command -v cvg || true
```

If `command -v` can’t find them, add `~/.local/bin` to your shell `PATH`.

If you installed from source via `scripts/install-local.sh`, binaries land in
`~/.cargo/bin`.

## 2) `scripts/install.sh` fails (missing `tar` / `curl` / SHA-256 tool)

The installer needs:

- `tar`
- `curl` or `wget`
- one of: `shasum` (macOS), `sha256sum` (Linux), or `openssl`

Run:

```bash
tar --version || true
command -v curl wget shasum sha256sum openssl | cat
```

Then retry the install.

## 3) `cvg doctor` says the daemon is unreachable

Start the daemon in one terminal:

```bash
convergio start
```

Or install + start the user-level service:

```bash
cvg service install
cvg service start
```

Then re-check:

```bash
cvg health
cvg doctor
```

## 4) Port `8420` is already in use

Run the daemon on another port:

```bash
convergio start --bind 127.0.0.1:8421
```

Point the CLI at that URL:

```bash
cvg --url http://127.0.0.1:8421 health
cvg --url http://127.0.0.1:8421 demo
```

## 5) SQLite permissions / “database is locked”

Convergio uses SQLite. The default state path is under `~/.convergio/`.

- If you run two daemons against the same DB, you can hit lock contention.
- If the directory is not writable, startup will fail.

To test with a fresh DB:

```bash
convergio start --db sqlite:///tmp/convergio.db?mode=rwc
```

## 6) macOS: “can’t be opened” / quarantined binary

Release artifacts may be **unsigned**. macOS Gatekeeper can quarantine
downloaded executables.

If you trust the binary and want to clear quarantine attributes:

```bash
xattr -dr com.apple.quarantine ~/.local/bin/convergio ~/.local/bin/cvg ~/.local/bin/convergio-mcp 2>/dev/null || true
```

Prefer signed/notarized artifacts when available. See `docs/release.md`.

## 7) CLI/daemon “version drift” warning

If `cvg` warns that its version doesn’t match the daemon, reinstall the
binaries and restart the service/daemon so both are updated.

```bash
cvg service restart
```

You can silence the warning with:

```bash
export CONVERGIO_NO_DRIFT_WARN=1
```

## 8) MCP bridge issues (agents)

Start with:

```bash
cvg mcp tail
cvg doctor --json
```

Host setup snippets live in `docs/agents/README.md`.

## 9) Unsupported platform for prebuilt binaries

Release artifacts are currently built for:

- `macos-arm64`
- `linux-x86_64`

If you’re on a different platform, build from source:

```bash
git clone https://github.com/Roberdan/convergio
cd convergio
sh scripts/install-local.sh
```

## 10) “`cvg demo` polluted my state”

`cvg demo` intentionally creates demo plans/tasks in your local SQLite state. That’s expected and safe to ignore.

## 11) Executor/dispatch fails with a worktree / repo_path error

Runner-based dispatch needs a repo root so Convergio can create a dedicated
agent worktree under `.claude/worktrees/…`. If you see an error mentioning
`worktree` or `repo_path`:

1. Run `cvg setup` from inside the repo you want Convergio to manage (it writes
   `repo_path` into `~/.convergio/config.toml`).
2. Or set `CONVERGIO_REPO_DIR=/path/to/your/repo` in your environment.

Then retry `cvg dispatch` / the executor loop.
