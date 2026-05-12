//! Theme resolver — decides how (and whether) to colour output.
//!
//! Resolution order, deterministic and easy to override:
//!
//! 1. `CONVERGIO_THEME=force-color` — **documented** intentional
//!    override that bypasses `NO_COLOR` (e.g. recording a demo).
//! 2. `NO_COLOR` env var (any value) — kill switch from
//!    [no-color.org]. Picks [`Theme::Mono`]. Defeats a casual
//!    `CONVERGIO_THEME=color` so the accessibility promise holds.
//! 3. `CONVERGIO_THEME=mono|color|hc` env var — explicit user
//!    choice (after `NO_COLOR`, except for `force-color`).
//! 4. `is_tty` — if stdout is not a TTY, default to [`Theme::Mono`]
//!    (keeps CI logs clean).
//! 5. Otherwise [`Theme::Color`].
//!
//! [no-color.org]: https://no-color.org

use std::env;

/// How brand surfaces should render colour and animation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    /// Full neon palette and animations. The default for an
    /// interactive terminal.
    Color,
    /// Plain ASCII, no escape sequences, no animation. The default
    /// for non-TTY stdout, CI logs, and `NO_COLOR=1`.
    Mono,
    /// White-on-black, bold-only, no gradients. For high-contrast
    /// accessibility setups.
    HighContrast,
}

impl Theme {
    /// Resolve the theme for an output stream, given whether that
    /// stream is currently a TTY. Pure: callers pass `is_tty`
    /// explicitly so tests do not depend on the environment.
    pub fn resolve(is_tty: bool) -> Self {
        // `force-color` is the documented intentional bypass of
        // NO_COLOR. Anything else from CONVERGIO_THEME is evaluated
        // *after* NO_COLOR so the P3 accessibility promise holds.
        let explicit = env::var("CONVERGIO_THEME").ok();
        if matches!(
            explicit.as_deref(),
            Some("force-color") | Some("force-colour")
        ) {
            return Theme::Color;
        }
        if env::var_os("NO_COLOR").is_some() {
            // NO_COLOR wins over a casual CONVERGIO_THEME=color, but
            // an explicit colourless choice (mono/hc) is still honoured.
            return match explicit.as_deref() {
                Some("hc") | Some("high-contrast") | Some("highcontrast") => Theme::HighContrast,
                _ => Theme::Mono,
            };
        }
        match explicit.as_deref() {
            Some("mono") | Some("plain") | Some("none") => return Theme::Mono,
            Some("hc") | Some("high-contrast") | Some("highcontrast") => {
                return Theme::HighContrast
            }
            Some("color") | Some("colour") => return Theme::Color,
            _ => {}
        }
        if !is_tty {
            return Theme::Mono;
        }
        Theme::Color
    }

    /// `true` when this theme allows truecolor escape sequences and
    /// gradients.
    pub fn allows_color(self) -> bool {
        matches!(self, Theme::Color)
    }

    /// ANSI prefix that wraps text for this theme when not using a
    /// per-character gradient. Empty string when no styling applies
    /// (i.e. plain mono). For [`Theme::HighContrast`] this is the
    /// documented bold + bright-white-on-black branch (SGR 1;97;40).
    pub fn style_prefix(self) -> &'static str {
        match self {
            Theme::HighContrast => "\x1b[1;97;40m",
            _ => "",
        }
    }

    /// ANSI suffix matching [`Self::style_prefix`]. Empty when no
    /// prefix is emitted.
    pub fn style_suffix(self) -> &'static str {
        match self {
            Theme::HighContrast => "\x1b[0m",
            _ => "",
        }
    }

    /// `true` when this theme allows boot animations (sleeps, glitch
    /// frames). Implies [`Self::allows_color`] today, but kept
    /// distinct so an operator can opt out of animation while
    /// keeping colour.
    pub fn allows_animation(self) -> bool {
        matches!(self, Theme::Color)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{catch_unwind, resume_unwind, UnwindSafe};
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Helper: resolve with a known TTY state inside an env-cleared
    /// scope. We do not parallelise these tests because they touch
    /// process-wide env vars.
    fn with_clean_env<F: FnOnce() -> R + UnwindSafe, R>(f: F) -> R {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let prev_theme = env::var("CONVERGIO_THEME").ok();
        let prev_no = env::var("NO_COLOR").ok();
        env::remove_var("CONVERGIO_THEME");
        env::remove_var("NO_COLOR");
        let result = catch_unwind(f);
        if let Some(v) = prev_theme {
            env::set_var("CONVERGIO_THEME", v);
        }
        if let Some(v) = prev_no {
            env::set_var("NO_COLOR", v);
        }
        match result {
            Ok(value) => value,
            Err(payload) => resume_unwind(payload),
        }
    }

    #[test]
    fn tty_defaults_to_color() {
        with_clean_env(|| {
            assert_eq!(Theme::resolve(true), Theme::Color);
        });
    }

    #[test]
    fn non_tty_defaults_to_mono() {
        with_clean_env(|| {
            assert_eq!(Theme::resolve(false), Theme::Mono);
        });
    }

    #[test]
    fn no_color_forces_mono_even_on_tty() {
        with_clean_env(|| {
            env::set_var("NO_COLOR", "1");
            assert_eq!(Theme::resolve(true), Theme::Mono);
            env::remove_var("NO_COLOR");
        });
    }

    #[test]
    fn no_color_wins_over_explicit_color_theme() {
        // P3 accessibility: NO_COLOR must defeat a casual
        // CONVERGIO_THEME=color, otherwise our "every output respects
        // NO_COLOR" promise (lib.rs) is a lie.
        with_clean_env(|| {
            env::set_var("NO_COLOR", "1");
            env::set_var("CONVERGIO_THEME", "color");
            assert_eq!(Theme::resolve(true), Theme::Mono);
            env::remove_var("CONVERGIO_THEME");
            env::remove_var("NO_COLOR");
        });
    }

    #[test]
    fn force_color_intentionally_bypasses_no_color() {
        // The documented escape hatch: an operator who *knows* what
        // they are doing can still force colour past NO_COLOR.
        with_clean_env(|| {
            env::set_var("NO_COLOR", "1");
            env::set_var("CONVERGIO_THEME", "force-color");
            assert_eq!(Theme::resolve(false), Theme::Color);
            env::remove_var("CONVERGIO_THEME");
            env::remove_var("NO_COLOR");
        });
    }

    #[test]
    fn no_color_does_not_break_explicit_high_contrast() {
        // HighContrast is bold-only / no truecolor, so NO_COLOR has
        // no reason to override it.
        with_clean_env(|| {
            env::set_var("NO_COLOR", "1");
            env::set_var("CONVERGIO_THEME", "hc");
            assert_eq!(Theme::resolve(true), Theme::HighContrast);
            env::remove_var("CONVERGIO_THEME");
            env::remove_var("NO_COLOR");
        });
    }

    #[test]
    fn high_contrast_alias_works() {
        with_clean_env(|| {
            env::set_var("CONVERGIO_THEME", "hc");
            assert_eq!(Theme::resolve(true), Theme::HighContrast);
            env::remove_var("CONVERGIO_THEME");
        });
    }

    #[test]
    fn allows_animation_only_in_color_theme() {
        assert!(Theme::Color.allows_animation());
        assert!(!Theme::Mono.allows_animation());
        assert!(!Theme::HighContrast.allows_animation());
    }
}
