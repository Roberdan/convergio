//! Regression tests for i18n diagnostics (P1).
//!
//! `Bundle::t` already emits a `tracing::warn!` when Fluent reports
//! format errors (e.g. a referenced placeholder is missing).
//! Historically `t_n` and `t_n_with` collected the same `Vec<FluentError>`
//! but threw it away, so plural-aware strings had weaker observability.
//! These tests exercise the warning surface for all three helpers.

use std::sync::{Arc, Mutex};

use convergio_i18n::{Bundle, Locale};
use tracing::subscriber::with_default;
use tracing_subscriber::fmt::MakeWriter;

#[derive(Clone, Default)]
struct CaptureWriter(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for CaptureWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for CaptureWriter {
    type Writer = CaptureWriter;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

fn capture<F: FnOnce()>(f: F) -> String {
    let writer = CaptureWriter::default();
    let buf = writer.0.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(writer)
        .with_max_level(tracing::Level::WARN)
        .with_ansi(false)
        .finish();
    with_default(subscriber, f);
    let bytes = buf.lock().unwrap().clone();
    String::from_utf8(bytes).unwrap()
}

/// `health-ok` references `{ $version }`. Calling `t_n` against it
/// means `$version` is never bound, so Fluent reports a format error.
/// `t_n` must surface that error via tracing, just like `t` does.
#[test]
fn t_n_logs_format_errors() {
    let logs = capture(|| {
        let b = Bundle::new(Locale::En).unwrap();
        let _ = b.t_n("health-ok", 1);
    });
    assert!(
        logs.contains("format errors"),
        "expected `t_n` to emit a `format errors` warning; got: {logs}"
    );
}

/// Same contract for `t_n_with`: a key whose pattern references an
/// argument we never bind must produce a tracing warning.
#[test]
fn t_n_with_logs_format_errors() {
    let logs = capture(|| {
        let b = Bundle::new(Locale::En).unwrap();
        let _ = b.t_n_with("health-ok", 1, &[]);
    });
    assert!(
        logs.contains("format errors"),
        "expected `t_n_with` to emit a `format errors` warning; got: {logs}"
    );
}

/// Missing-message diagnostics should also be consistent across the
/// three helpers — `t` warns, so `t_n` and `t_n_with` should too.
#[test]
fn missing_key_warns_consistently() {
    let logs_t = capture(|| {
        let b = Bundle::new(Locale::En).unwrap();
        let _ = b.t("definitely-missing-key", &[]);
    });
    let logs_tn = capture(|| {
        let b = Bundle::new(Locale::En).unwrap();
        let _ = b.t_n("definitely-missing-key", 1);
    });
    let logs_tnw = capture(|| {
        let b = Bundle::new(Locale::En).unwrap();
        let _ = b.t_n_with("definitely-missing-key", 1, &[]);
    });
    assert!(logs_t.contains("missing message"));
    assert!(
        logs_tn.contains("missing message"),
        "`t_n` must warn on missing keys; got: {logs_tn}"
    );
    assert!(
        logs_tnw.contains("missing message"),
        "`t_n_with` must warn on missing keys; got: {logs_tnw}"
    );
}
