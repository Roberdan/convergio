//! Focus movement, row movement, and drill-down scope application.

use crate::client::Client;
use crate::state::{AppMode, AppState, DetailTarget, Pane, Scope};

impl AppState {
    /// Move focus to the next pane in tab order.
    pub fn focus_next(&mut self) {
        let idx = Pane::ALL.iter().position(|p| *p == self.focus).unwrap_or(0);
        self.focus = Pane::ALL[(idx + 1) % Pane::ALL.len()];
    }

    /// Move focus to the previous pane.
    pub fn focus_prev(&mut self) {
        let idx = Pane::ALL.iter().position(|p| *p == self.focus).unwrap_or(0);
        self.focus = Pane::ALL[(idx + Pane::ALL.len() - 1) % Pane::ALL.len()];
    }

    /// Cursor down within the focused pane.
    pub fn row_down(&mut self) {
        let len = self.focused_len();
        self.focused_cursor_mut().down(len, 8);
    }

    /// Cursor up within the focused pane.
    pub fn row_up(&mut self) {
        self.focused_cursor_mut().up();
    }

    /// Apply the focused row as the cross-pane drill-down scope.
    pub fn apply_scope_from_focus(&mut self) {
        let next = match self.focus {
            Pane::Plans => self
                .plans
                .get(self.cursor.plans.selected)
                .map(|p| Scope::Plan {
                    id: p.id.clone(),
                    title: p.title.clone(),
                }),
            Pane::Tasks => self
                .scoped_tasks()
                .get(self.cursor.tasks.selected)
                .map(|t| Scope::Task {
                    id: t.id.clone(),
                    plan_id: t.plan_id.clone(),
                    title: t.title.clone(),
                }),
            Pane::Agents => self
                .scoped_agent_processes()
                .get(self.cursor.agents.selected)
                .map(|a| Scope::Agent { id: a.id.clone() })
                .or_else(|| {
                    self.scoped_agents()
                        .get(self.cursor.agents.selected)
                        .map(|a| Scope::Agent { id: a.id.clone() })
                }),
            Pane::Prs => self
                .scoped_prs()
                .get(self.cursor.prs.selected)
                .map(|p| Scope::Pr {
                    number: p.number,
                    title: p.title.clone(),
                }),
            Pane::Bus => None,
        };
        if let Some(scope) = next {
            self.scope = scope;
            self.reset_dependent_cursors();
        }
    }

    /// Clear any drill-down scope. Returns true when a scope was cleared.
    pub fn clear_scope(&mut self) -> bool {
        if self.scope == Scope::All {
            return false;
        }
        self.scope = Scope::All;
        self.reset_dependent_cursors();
        true
    }

    /// Build the detail target for the focused row.
    pub fn drill_target(&self) -> Option<DetailTarget> {
        match self.focus {
            Pane::Plans => self
                .plans
                .get(self.cursor.plans.selected)
                .map(|p| DetailTarget::Plan {
                    id: p.id.clone(),
                    title: p.title.clone(),
                }),
            Pane::Tasks => self
                .scoped_tasks()
                .get(self.cursor.tasks.selected)
                .map(|t| DetailTarget::Task {
                    id: t.id.clone(),
                    plan_id: t.plan_id.clone(),
                    title: t.title.clone(),
                }),
            Pane::Agents => self
                .scoped_agent_processes()
                .get(self.cursor.agents.selected)
                .map(|a| DetailTarget::Agent { id: a.id.clone() })
                .or_else(|| {
                    self.scoped_agents()
                        .get(self.cursor.agents.selected)
                        .map(|a| DetailTarget::Agent { id: a.id.clone() })
                }),
            Pane::Prs => {
                self.scoped_prs()
                    .get(self.cursor.prs.selected)
                    .map(|p| DetailTarget::Pr {
                        number: p.number,
                        title: p.title.clone(),
                    })
            }
            Pane::Bus => self
                .scoped_messages()
                .get(self.cursor.bus.selected)
                .map(|m| DetailTarget::BusMessage {
                    id: m.id.clone(),
                    seq: m.seq,
                    topic: m.topic.clone(),
                }),
        }
    }

    /// Enter legacy full-screen detail mode.
    pub async fn enter_detail(&mut self, client: &Client, target: DetailTarget) {
        if let DetailTarget::Plan { id, .. } = &target {
            self.detail_tasks = client.fetch_plan_tasks(id).await.unwrap_or_default();
        } else {
            self.detail_tasks.clear();
        }
        self.mode = AppMode::Detail(target);
    }

    /// Leave detail mode and return to the overview.
    pub fn back_to_overview(&mut self) {
        self.mode = AppMode::Overview;
        self.detail_tasks.clear();
    }

    fn focused_len(&self) -> usize {
        match self.focus {
            Pane::Plans => self.plans.len(),
            Pane::Tasks => self.scoped_tasks().len(),
            Pane::Agents => {
                let processes = self.scoped_agent_processes().len();
                if processes == 0 {
                    self.scoped_agents().len()
                } else {
                    processes
                }
            }
            Pane::Prs => self.scoped_prs().len(),
            Pane::Bus => self.scoped_messages().len(),
        }
    }

    fn focused_cursor_mut(&mut self) -> &mut crate::state::Cursor {
        match self.focus {
            Pane::Plans => &mut self.cursor.plans,
            Pane::Tasks => &mut self.cursor.tasks,
            Pane::Agents => &mut self.cursor.agents,
            Pane::Prs => &mut self.cursor.prs,
            Pane::Bus => &mut self.cursor.bus,
        }
    }

    fn reset_dependent_cursors(&mut self) {
        self.cursor.tasks.selected = 0;
        self.cursor.tasks.offset = 0;
        self.cursor.agents.selected = 0;
        self.cursor.agents.offset = 0;
        self.cursor.prs.selected = 0;
        self.cursor.prs.offset = 0;
        self.cursor.bus.selected = 0;
        self.cursor.bus.offset = 0;
    }
}
