//! Binary-vs-daemon version drift helper.
//!
//! Split out of [`crate::state`] so that file stays under the
//! 300-line cap. The dashboard header surfaces drift between the
//! `cvg` binary embedding this dashboard and the live daemon it talks
//! to, so an operator running a stale binary is told to `cvg update`.

/// Compile-time version of the `cvg` binary embedding this dashboard.
/// Compared against the live daemon version to surface drift.
pub const BINARY_VERSION: &str = env!("CARGO_PKG_VERSION");

/// `Some(daemon)` when the daemon and the binary report different
/// versions, `None` when they match or the daemon is unreachable.
pub fn version_drift(daemon: Option<&str>) -> Option<String> {
    let d = daemon?;
    if d == BINARY_VERSION {
        None
    } else {
        Some(d.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_drift_when_versions_match() {
        assert_eq!(version_drift(Some(BINARY_VERSION)), None);
    }

    #[test]
    fn drift_reported_when_daemon_differs() {
        assert_eq!(
            version_drift(Some("0.0.0-other")),
            Some("0.0.0-other".into())
        );
    }

    #[test]
    fn no_drift_when_daemon_unreachable() {
        assert_eq!(version_drift(None), None);
    }
}
