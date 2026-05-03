# Convergio brand kit

Source assets for the Convergio visual identity. Anyone rebuilding
marketing surfaces (web, deck, print) starts here.

## Claim

> **Make machines prove it.**
>
> The machine that builds machines — and proves they work.

The claim is also encoded as constants in
[`crates/convergio-brand/src/claim.rs`](../../crates/convergio-brand/src/claim.rs)
and consumed by `cvg about`, the daemon boot banner, and the TUI.

## Palette

| Token | Hex | RGB |
|-------|-----|-----|
| Brand magenta | `#FF00B4` | `255, 0, 180` |
| Brand cyan | `#00C8FF` | `0, 200, 255` |
| Brand ground | `#000000` | `0, 0, 0` |

The palette lives in code at
[`crates/convergio-brand/src/palette.rs`](../../crates/convergio-brand/src/palette.rs).

## Files

| File | What it is |
|------|------------|
| `lockup-hex-wordmark.png` | Hexagonal-C mark stacked above the wordmark — primary lockup |
| `wordmark-neon.png` | Wordmark only, neon variant |
| `wordmark-pixel.png` | Wordmark only, pixel/scanline variant |
| `icon-hex.png` | Hexagonal-C mark, large |
| `icon-small.png` | Hexagonal-C mark, small (favicon-grade) |
| `cli-mockup.png` | Mockup of the CLI splash (reference for `cvg about`) |
| `convergio.sh` | Original Bash demo of the boot animation |
| `cli_monitor.sh` | Bash mock of `cvg status` (reference) |
| `demo-cli/` | Bash kit (`banner.sh`, `boot.sh`, `colors.sh`, `glitch.sh`) |
| `demo-rust/` | Standalone Rust demo of the gradient + glitch |

The Rust runtime equivalent of `demo-rust/` is the
`convergio-brand` crate — that crate is what the CLI, TUI, and
daemon actually link against. The folder above is kept verbatim
as a reference so future visual updates can be diffed against it.

## Re-exports

| Surface | Sources from |
|---------|--------------|
| `cvg about` | `convergio-brand` (claim, banner, boot) |
| `convergio start` boot banner | `convergio-brand` (boot, theme) |
| `cvg dash` header gradient | `convergio-brand` (palette) |
| README header | `lockup-hex-wordmark.png` |

If you need to change the palette or the claim, change it in
`convergio-brand` and every surface above updates on rebuild.
