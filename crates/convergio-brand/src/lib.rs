//! # convergio-brand — Convergio brand kit
//!
//! Shared visual identity used by every user-facing surface in
//! Convergio: the `cvg` CLI, the `cvg dash` TUI, and the daemon
//! boot banner.
//!
//! The brand kit is **claim-first**: the product makes one promise
//! and we say it the same way everywhere.
//!
//! - Claim: [`CLAIM`] — *Make machines prove it.*
//! - Subline: [`SUBLINE`] — *The machine that builds machines — and
//!   proves they work.*
//!
//! ## What lives here
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | [`palette`] | Magenta / cyan / black truecolor constants |
//! | [`claim`] | Claim and subline constants |
//! | [`theme`] | TTY / `NO_COLOR` / high-contrast resolver |
//! | [`gradient`] | RGB interpolation + truecolor escape sequences |
//! | [`glitch`] | Glitch char-swap helper for the boot animation |
//! | [`banner`] | ASCII wordmark + hexagonal lockup |
//! | [`boot`] | Boot animation orchestrator (CLI + daemon) |
//!
//! ## Accessibility (CONSTITUTION P3)
//!
//! Every output respects `NO_COLOR` (per the `no-color.org`
//! convention) and disables animation when stdout is not a TTY. The
//! [`theme::Theme`] enum exposes a `HighContrast` variant for
//! operators who need pure white-on-black.
//!
//! ## i18n (CONSTITUTION P5)
//!
//! Brand marks (claim, subline, product name) are **not**
//! translated — they are part of the trade dress. Surrounding
//! prose (help text, labels) flows through `convergio-i18n`.

#![forbid(unsafe_code)]

pub mod banner;
pub mod boot;
pub mod claim;
pub mod glitch;
pub mod gradient;
pub mod palette;
pub mod theme;

pub use claim::{CLAIM, PRODUCT_NAME, SUBLINE};
pub use palette::{Rgb, BLACK, CYAN, MAGENTA};
pub use theme::Theme;
