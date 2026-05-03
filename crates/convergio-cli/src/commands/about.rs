//! `cvg about` — print the brand lockup, claim, and subline.
//!
//! Plays the boot animation when stdout is a TTY (and `NO_COLOR`
//! is not set), prints the static lockup otherwise. Honors
//! `CONVERGIO_THEME=mono|color|hc` for explicit overrides.

use std::io::{self, IsTerminal, Write};

use anyhow::Result;
use convergio_brand::{banner, boot, theme::Theme};
use convergio_i18n::Bundle;

/// Run `cvg about`. The `animate` flag forces the boot animation
/// even if the theme would skip it (still respects `NO_COLOR`).
/// Translated labels flow through [`Bundle`]; brand marks (claim,
/// subline, product name) come straight from `convergio_brand` and
/// are not translated.
pub fn run(bundle: &Bundle, animate: bool) -> Result<()> {
    let stdout = io::stdout();
    let is_tty = stdout.is_terminal();
    let theme = Theme::resolve(is_tty);
    let mut handle = stdout.lock();
    if animate || theme.allows_animation() {
        let mut sleeper = boot::RealSleeper;
        boot::play(&mut handle, &mut sleeper, theme)?;
    } else {
        writeln!(handle, "{}", banner::lockup(theme))?;
    }
    writeln!(handle)?;
    let version = env!("CARGO_PKG_VERSION");
    writeln!(
        handle,
        "  {}",
        bundle.t("brand-about-tagline", &[("version", version)])
    )?;
    writeln!(
        handle,
        "  {}",
        bundle.t(
            "brand-about-source",
            &[("url", "https://github.com/Roberdan/convergio")]
        )
    )?;
    writeln!(handle, "  {}", bundle.t("brand-about-help", &[]))?;
    handle.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use convergio_i18n::Locale;

    #[test]
    fn run_does_not_panic() {
        // Stdout is captured in tests; this just makes sure the
        // function returns Ok without exploding on any path.
        let bundle = Bundle::new(Locale::En).expect("load CLI Fluent bundle");
        let _ = run(&bundle, false);
    }
}
