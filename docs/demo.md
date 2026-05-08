---
topic: demo
---

# Demo (recording + replay)

Convergio ships a guided local demo (`cvg demo`). It creates:

1. a deliberately dirty task that should be refused by gates (debt marker)
2. a clean task that should be accepted, then validated by Thor

## Run the demo

With the daemon running:

```bash
cvg demo
```

You should see a gate refusal, then a clean plan validation verdict and an
`audit verify` result.

## Replay the recorded terminal demo

A short terminal recording is stored at:

- `docs/demo/quickstart.cast`

To play it you need `asciinema`.

On macOS:

```bash
brew install asciinema
```

On Debian/Ubuntu:

```bash
sudo apt-get update && sudo apt-get install -y asciinema
```

Then:

```bash
asciinema play docs/demo/quickstart.cast
```

## Re-record (for maintainers)

This records `cvg demo` against your current running daemon.

Note: `cvg demo` creates demo plans/tasks in your local SQLite state
(default: `~/.convergio/v3/state.db`). That’s expected.

```bash
CONVERGIO_NO_DRIFT_WARN=1 asciinema rec -q -c 'cvg demo' docs/demo/quickstart.cast
```
