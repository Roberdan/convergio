//! UI mode (Overview vs drill-down) and the entity targeted by the
//! drill.
//!
//! Split out of [`crate::state`] so the file stays under the
//! 300-line cap. The flow is:
//!
//! 1. The keymap (`crate::keymap::Action::Drill`) tells the event
//!    loop the user pressed Enter.
//! 2. [`crate::state::AppState::drill_target`] inspects the focused
//!    pane + selected row and produces a [`DetailTarget`].
//! 3. [`crate::state::AppState::enter_detail`] flips
//!    [`crate::state::AppState::mode`] to [`AppMode::Detail`] and
//!    pre-fetches whatever extra data the renderer needs.
//! 4. The [`crate::panes::detail`] renderer takes over the body
//!    until the user presses Esc.

/// Top-level UI mode.
///
/// `Overview` is the default 4-pane dashboard. `Detail` zooms into a
/// single entity (plan / task / agent / PR) and replaces the body
/// with a single full-width panel — the panes header + footer stay so
/// the operator can still see plan counts at a glance.
#[derive(Debug, Default, Clone)]
pub enum AppMode {
    /// 4-pane overview.
    #[default]
    Overview,
    /// Drill-down into one entity.
    Detail(DetailTarget),
}

/// Which entity is being drilled into.
#[derive(Debug, Clone)]
pub enum DetailTarget {
    /// A plan, by id. Includes the title for header rendering even
    /// when the underlying plan list refreshes mid-detail.
    Plan {
        /// Plan id.
        id: String,
        /// Plan title at the moment drill-down started.
        title: String,
    },
    /// A single task. Title preserved for the detail header.
    Task {
        /// Task id.
        id: String,
        /// Plan id this task belongs to (for crumb display).
        plan_id: String,
        /// Task title.
        title: String,
    },
    /// A registered agent.
    Agent {
        /// Agent id.
        id: String,
    },
    /// An open pull request.
    Pr {
        /// PR number.
        number: i64,
        /// PR title.
        title: String,
    },
    /// A single bus message — full payload + headers in the detail
    /// panel. Stored by stable id so the detail keeps rendering even
    /// if the buffer rolls past the row.
    BusMessage {
        /// Message id (stable across reads).
        id: String,
        /// Sequence number for the title crumb.
        seq: i64,
        /// Topic captured at drill time.
        topic: String,
    },
}

/// Cross-pane drill-down scope.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum Scope {
    /// Nothing selected: every pane shows its whole dataset.
    #[default]
    All,
    /// Filter dashboard data to one plan.
    Plan {
        /// Plan id.
        id: String,
        /// Plan title.
        title: String,
    },
    /// Filter dashboard data to one task.
    Task {
        /// Task id.
        id: String,
        /// Owning plan id.
        plan_id: String,
        /// Task title.
        title: String,
    },
    /// Filter dashboard data to one supervised agent process.
    Agent {
        /// Process or agent id.
        id: String,
    },
    /// Filter dashboard data to one PR.
    Pr {
        /// PR number.
        number: i64,
        /// PR title.
        title: String,
    },
}
