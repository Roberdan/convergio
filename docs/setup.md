---
topic: setup
---

# Local setup

Convergio v3 is a single-user local daemon. It needs no account, no
Postgres, and no external service.

## Install

### Prebuilt binaries (recommended)

Supported today: macOS arm64 and Linux x86_64.

```bash
curl -fsSL https://raw.githubusercontent.com/Roberdan/convergio/main/scripts/install.sh | sh
```

By default this installs into `~/.local/bin`. Override with `--dir <path>` or
`CONVERGIO_INSTALL_DIR=/path`.

Install a specific release tag:

```bash
curl -fsSL https://raw.githubusercontent.com/Roberdan/convergio/main/scripts/install.sh | sh -s -- --tag <tag>
```

See installer options:

```bash
curl -fsSL https://raw.githubusercontent.com/Roberdan/convergio/main/scripts/install.sh | sh -s -- --help
```

If you prefer not piping to `sh`, download then run:

```bash
curl -fsSL https://raw.githubusercontent.com/Roberdan/convergio/main/scripts/install.sh \
  -o /tmp/convergio-install.sh
sh /tmp/convergio-install.sh
```

### From source (Rust)

```bash
git clone https://github.com/Roberdan/convergio
cd convergio
sh scripts/install-local.sh
```

## Run

```bash
cvg setup
convergio start
```

In another terminal:

```bash
cvg doctor
cvg health
cvg demo
```

If you get stuck, see [troubleshooting.md](./troubleshooting.md).

To install the daemon as a user-level service:

```bash
cvg service install
cvg service start
cvg service status
```

The default state lives under `~/.convergio/`:

| Path | Purpose |
|------|---------|
| `config.toml` | local URL, bind address, SQLite URL |
| `v3/state.db` | SQLite database |
| `daemon.pid` | daemon discovery for `cvg doctor` |
| `adapters/` | generated agent snippets |
| `mcp.log` | compact MCP action diagnostics |

## Release artifacts

CI builds Linux and macOS tarballs on release tags. They are unsigned
unless signing/notarization credentials are configured. Locally, you can
produce the same shape with:

```bash
sh scripts/package-local.sh
```

macOS signing and notarization are intentionally not faked in this repo.
Use the documented scripts with real Apple credentials when producing
signed public artifacts.

On a Mac with a Developer ID Application certificate installed, sign a
local package with:

```bash
sh scripts/package-local.sh
sh scripts/sign-macos-local.sh
```

For notarization, provide either a notarytool keychain profile:

```bash
APPLE_NOTARY_PROFILE=convergio-notary sh scripts/sign-macos-local.sh
```

or App Store Connect API key variables:

```bash
APPLE_API_KEY_PATH=/path/AuthKey_XXXX.p8 \
APPLE_API_KEY_ID=XXXX \
APPLE_API_ISSUER_ID=xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx \
sh scripts/sign-macos-local.sh
```

See [release.md](./release.md) for the full repeatable release,
signing, and notarization workflow.

## Agent setup

Generate host-specific snippets:

```bash
cvg setup agent claude
cvg setup agent cursor
cvg setup agent qwen
```

Each command writes `mcp.json`, `prompt.txt`, and `README.txt` under
`~/.convergio/adapters/<host>/`. Copy `mcp.json` into the host's MCP
configuration and `prompt.txt` into its custom instructions.

All snippets use the same bridge:

```bash
convergio-mcp --url http://127.0.0.1:8420
```

If an agent cannot connect, run:

```bash
cvg doctor --json
cvg mcp tail
```
