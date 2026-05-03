//! `cvg monitor` — live tail of the daemon's audit log.
//!
//! Streams every state transition (task moves, evidence drops,
//! refusals, agent heartbeats, bus messages) as it happens, with
//! brand-coloured glyphs so the operator can see refusals stand out
//! at a glance.
//!
//! Polls `/v1/audit/events?after_seq=…` on a tick. Cursor is the
//! last seen `seq`. Exit on Ctrl-C.

use std::io::{self, IsTerminal, Write};
use std::time::Duration;

use anyhow::Result;
use convergio_brand::{banner, theme::Theme};
use convergio_durability::audit::AuditEntry;

use super::Client;

/// Run the monitor loop until interrupted.
pub async fn run(client: &Client, tick_secs: u64) -> Result<()> {
    let tick = tick_secs.clamp(1, 60);
    let theme = Theme::resolve(io::stdout().is_terminal());
    print_header(theme)?;

    let mut after_seq: i64 = current_tail(client).await.unwrap_or(0);
    let mut interval = tokio::time::interval(Duration::from_secs(tick));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        interval.tick().await;
        let url = format!("/v1/audit/events?after_seq={after_seq}&limit=200");
        let entries: Vec<AuditEntry> = match client.get(&url).await {
            Ok(v) => v,
            Err(_) => continue, // daemon hiccup; keep polling
        };
        for entry in entries {
            after_seq = after_seq.max(entry.seq);
            print_entry(&entry, theme)?;
        }
    }
}

async fn current_tail(client: &Client) -> Option<i64> {
    // Start the cursor at the current tail so the operator sees only
    // events that happen *after* `cvg monitor` begins. Dropped first
    // batch is intentional — operators do not want a wall of
    // historical noise on every launch.
    let entries: Vec<AuditEntry> = client
        .get("/v1/audit/events?after_seq=0&limit=1000")
        .await
        .ok()?;
    entries.into_iter().map(|e| e.seq).max()
}

fn print_header(theme: Theme) -> Result<()> {
    let stdout = io::stdout();
    let mut h = stdout.lock();
    writeln!(h, "{}", banner::lockup(theme))?;
    let label = "  Live audit stream — Ctrl-C to exit.";
    if theme.allows_color() {
        writeln!(h, "\x1b[2m{label}\x1b[0m")?;
    } else {
        writeln!(h, "{label}")?;
    }
    writeln!(h)?;
    h.flush()?;
    Ok(())
}

fn print_entry(entry: &AuditEntry, theme: Theme) -> Result<()> {
    let stdout = io::stdout();
    let mut h = stdout.lock();
    let glyph = glyph_for(&entry.transition);
    let agent = entry.agent_id.as_deref().unwrap_or("-");
    let ts = entry
        .created_at
        .split('T')
        .nth(1)
        .unwrap_or(&entry.created_at);
    let ts = ts.split('.').next().unwrap_or(ts); // drop subsec / TZ
    let line = format!(
        "{ts}  {glyph}  {transition:24}  {entity:>12}={id}  agent={agent}",
        transition = entry.transition,
        entity = entry.entity_type,
        id = short(&entry.entity_id),
    );
    if theme.allows_color() {
        let color = colour_for(&entry.transition);
        writeln!(
            h,
            "\x1b[38;2;{};{};{}m{line}\x1b[0m",
            color.0, color.1, color.2
        )?;
    } else {
        writeln!(h, "{line}")?;
    }
    h.flush()?;
    Ok(())
}

fn short(id: &str) -> String {
    if id.len() <= 8 {
        id.to_string()
    } else {
        format!("{}…", &id[..8])
    }
}

fn glyph_for(transition: &str) -> &'static str {
    if transition.contains("refused") || transition.contains("failed") {
        "✗"
    } else if transition.contains("done") || transition.contains("validated") {
        "✓"
    } else if transition.contains("submitted") || transition.contains("transition") {
        "▶"
    } else if transition.contains("evidence") {
        "◆"
    } else if transition.contains("reaped") || transition.contains("cancelled") {
        "⊘"
    } else {
        "·"
    }
}

fn colour_for(transition: &str) -> (u8, u8, u8) {
    if transition.contains("refused") || transition.contains("failed") {
        (255, 0, 180) // brand magenta — refusals stand out
    } else if transition.contains("done") || transition.contains("validated") {
        (0, 200, 255) // brand cyan — happy path
    } else if transition.contains("submitted") || transition.contains("transition") {
        (192, 202, 245) // info text
    } else {
        (169, 177, 214) // dim
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glyph_for_refusal_is_x() {
        assert_eq!(glyph_for("task.refused"), "✗");
        assert_eq!(glyph_for("gate.failed"), "✗");
    }

    #[test]
    fn glyph_for_happy_path_is_check() {
        assert_eq!(glyph_for("task.done"), "✓");
        assert_eq!(glyph_for("plan.validated"), "✓");
    }

    #[test]
    fn glyph_for_unknown_is_dot() {
        assert_eq!(glyph_for("something.unknown"), "·");
    }

    #[test]
    fn short_truncates_long_ids() {
        assert_eq!(short("abcdefghij"), "abcdefgh…");
    }

    #[test]
    fn short_passes_through_ids_under_cap() {
        assert_eq!(short("abc"), "abc");
        assert_eq!(short("12345678"), "12345678");
    }

    #[test]
    fn colour_for_refusal_is_brand_magenta() {
        assert_eq!(colour_for("task.refused"), (255, 0, 180));
    }

    #[test]
    fn colour_for_done_is_brand_cyan() {
        assert_eq!(colour_for("task.done"), (0, 200, 255));
    }
}
