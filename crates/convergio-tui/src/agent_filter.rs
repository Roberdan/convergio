//! Agent visibility filters for the dashboard (P2-11).
//!
//! Pure helpers split out of `scope.rs` so that crate stays under
//! the 300-line cap while still letting tests exercise the
//! exited-agent toggle in isolation.

use crate::client::RegistryAgent;

/// Canonical statuses that mark an agent as no longer participating.
/// Hidden by default in the Agents pane.
pub fn is_exited_status(status: Option<&str>) -> bool {
    matches!(
        status,
        Some("terminated") | Some("retired") | Some("exited")
    )
}

/// Drop terminal-status agents from `candidates` unless `show_exited`
/// is `true`. Pure function, no allocation when nothing is dropped.
pub fn filter_exited(candidates: Vec<&RegistryAgent>, show_exited: bool) -> Vec<&RegistryAgent> {
    if show_exited {
        return candidates;
    }
    candidates
        .into_iter()
        .filter(|a| !is_exited_status(a.status.as_deref()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk(status: &str) -> RegistryAgent {
        RegistryAgent {
            id: status.into(),
            kind: "claude".into(),
            status: Some(status.into()),
            ..RegistryAgent::default()
        }
    }

    #[test]
    fn is_exited_status_recognises_terminal_states() {
        assert!(is_exited_status(Some("terminated")));
        assert!(is_exited_status(Some("retired")));
        assert!(is_exited_status(Some("exited")));
        assert!(!is_exited_status(Some("idle")));
        assert!(!is_exited_status(Some("working")));
        assert!(!is_exited_status(None));
    }

    #[test]
    fn filter_exited_drops_terminal_when_hidden() {
        let agents = [mk("idle"), mk("terminated"), mk("retired"), mk("working")];
        let refs: Vec<&RegistryAgent> = agents.iter().collect();
        let kept: Vec<&str> = filter_exited(refs, false)
            .into_iter()
            .map(|a| a.id.as_str())
            .collect();
        assert_eq!(kept, ["idle", "working"]);
    }

    #[test]
    fn filter_exited_passes_through_when_revealed() {
        let agents = [mk("idle"), mk("terminated")];
        let refs: Vec<&RegistryAgent> = agents.iter().collect();
        let kept: Vec<&str> = filter_exited(refs, true)
            .into_iter()
            .map(|a| a.id.as_str())
            .collect();
        assert_eq!(kept, ["idle", "terminated"]);
    }
}
