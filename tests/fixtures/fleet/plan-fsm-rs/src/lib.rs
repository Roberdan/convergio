//! Plan finite-state machine — Rust reference implementation.
//!
//! Deliberate duplicate of the logic in `plan-fsm-ts` and `plan-fsm-py`.
//! Used by convergio-fleet cross-language fixture tests (F2-11).

/// States of a plan in the Convergio lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanState {
    Pending,
    InProgress,
    Done,
    Failed,
}

/// Attempt a state transition. Returns `None` for invalid transitions.
pub fn transition(state: &PlanState, event: &str) -> Option<PlanState> {
    match (state, event) {
        (PlanState::Pending, "start") => Some(PlanState::InProgress),
        (PlanState::InProgress, "complete") => Some(PlanState::Done),
        (PlanState::InProgress, "fail") => Some(PlanState::Failed),
        _ => None,
    }
}

/// Returns `true` for terminal states where no further transition is possible.
pub fn is_terminal(state: &PlanState) -> bool {
    matches!(state, PlanState::Done | PlanState::Failed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_to_in_progress() {
        assert_eq!(
            transition(&PlanState::Pending, "start"),
            Some(PlanState::InProgress)
        );
    }

    #[test]
    fn in_progress_to_done() {
        assert_eq!(
            transition(&PlanState::InProgress, "complete"),
            Some(PlanState::Done)
        );
    }

    #[test]
    fn in_progress_to_failed() {
        assert_eq!(
            transition(&PlanState::InProgress, "fail"),
            Some(PlanState::Failed)
        );
    }

    #[test]
    fn invalid_transition_returns_none() {
        assert_eq!(transition(&PlanState::Done, "start"), None);
    }

    #[test]
    fn done_is_terminal() {
        assert!(is_terminal(&PlanState::Done));
        assert!(is_terminal(&PlanState::Failed));
        assert!(!is_terminal(&PlanState::Pending));
        assert!(!is_terminal(&PlanState::InProgress));
    }
}
