//! Theme resolver — decides how (and whether) to colour output.
//!
//! Resolution order, deterministic and easy to override:
//!
//! 1. `CONVERGIO_THEME=mono|color|hc` env var — explicit user choice.
//! 2. `NO_COLOR` env var (any value) — kill switch from
//!    [no-color.org]. Picks [`Theme::Mono`].
//! 3. `is_tty` — if stdout is not a TTY, default to [`Theme::Mono`]
//!    (keeps CI logs clean).
//! 4. Otherwise [`Theme::Color`].
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
        match env::var("CONVERGIO_THEME").ok().as_deref() {
            Some("mono") | Some("plain") | Some("none") => return Theme::Mono,
            Some("hc") | Some("high-contrast") | Some("highcontrast") => {
                return Theme::HighContrast
            }
            Some("color") | Some("colour") => return Theme::Color,
            _ => {}
        }
        if env::var_os("NO_COLOR").is_some() {
            return Theme::Mono;
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

    /// Helper: resolve with a known TTY state inside an env-cleared
    /// scope. We do not parallelise these tests because they touch
    /// process-wide env vars.
    fn with_clean_env<F: FnOnce() -> R, R>(f: F) -> R {
        let prev_theme = env::var("CONVERGIO_THEME").ok();
        let prev_no = env::var("NO_COLOR").ok();
        env::remove_var("CONVERGIO_THEME");
        env::remove_var("NO_COLOR");
        let r = f();
        if let Some(v) = prev_theme {
            env::set_var("CONVERGIO_THEME", v);
        }
        if let Some(v) = prev_no {
            env::set_var("NO_COLOR", v);
        }
        r
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
    fn explicit_theme_overrides_no_color() {
        with_clean_env(|| {
            env::set_var("NO_COLOR", "1");
            env::set_var("CONVERGIO_THEME", "color");
            assert_eq!(Theme::resolve(false), Theme::Color);
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
