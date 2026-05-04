//! Cross-pane scope filtering.
//!
//! `Enter` on a plan/task/agent/PR sets a cross-pane scope. Startup
//! scope is [`Scope::All`], so every pane initially shows everything.

use crate::client::{AgentProcess, BusMessage, PrSummary, RegistryAgent, TaskSummary};
use crate::state::{AppState, Scope};
use std::collections::HashSet;

impl AppState {
    /// Plan id currently scoped, if any.
    pub fn scoped_plan_id(&self) -> Option<&str> {
        match &self.scope {
            Scope::Plan { id, .. } => Some(id.as_str()),
            Scope::Task { plan_id, .. } => Some(plan_id.as_str()),
            _ => None,
        }
    }

    /// Human-readable scope label for pane titles.
    pub fn scoped_plan_title(&self) -> Option<&str> {
        match &self.scope {
            Scope::Plan { title, .. } | Scope::Task { title, .. } | Scope::Pr { title, .. } => {
                Some(title.as_str())
            }
            Scope::Agent { id } => Some(id.as_str()),
            Scope::All => None,
        }
    }

    /// Tasks visible under the current scope.
    pub fn scoped_tasks(&self) -> Vec<TaskSummary> {
        match &self.scope {
            Scope::All => self.tasks.clone(),
            Scope::Plan { id, .. } => self
                .tasks
                .iter()
                .filter(|t| t.plan_id == *id)
                .cloned()
                .collect(),
            Scope::Task { id, .. } => self.tasks.iter().filter(|t| t.id == *id).cloned().collect(),
            Scope::Agent { id } => {
                let task_ids = self
                    .agent_processes
                    .iter()
                    .filter(|a| a.id == *id)
                    .filter_map(|a| a.task_id.as_deref())
                    .collect::<HashSet<_>>();
                self.tasks
                    .iter()
                    .filter(|t| {
                        t.agent_id.as_deref() == Some(id.as_str())
                            || task_ids.contains(t.id.as_str())
                    })
                    .cloned()
                    .collect()
            }
            Scope::Pr { number, .. } => {
                let linked = self.pr_task_ids(*number);
                self.tasks
                    .iter()
                    .filter(|t| linked.contains(t.id.as_str()))
                    .cloned()
                    .collect()
            }
        }
    }

    /// Registered agents visible under the current scope.
    pub fn scoped_agents(&self) -> Vec<&RegistryAgent> {
        let tasks = self.scoped_tasks();
        let task_ids = tasks.iter().map(|t| t.id.as_str()).collect::<HashSet<_>>();
        let owners = tasks
            .iter()
            .filter_map(|t| t.agent_id.as_deref())
            .collect::<HashSet<_>>();
        match self.scope {
            Scope::All => self.agents.iter().collect(),
            _ => self
                .agents
                .iter()
                .filter(|a| {
                    owners.contains(a.id.as_str())
                        || a.current_task_id
                            .as_deref()
                            .map(|id| task_ids.contains(id))
                            .unwrap_or(false)
                })
                .collect(),
        }
    }

    /// Supervised agent processes visible under the current scope.
    pub fn scoped_agent_processes(&self) -> Vec<AgentProcess> {
        let tasks = self.scoped_tasks();
        let task_ids = tasks.iter().map(|t| t.id.as_str()).collect::<HashSet<_>>();
        match &self.scope {
            Scope::All => self.agent_processes.clone(),
            Scope::Plan { id, .. } => self
                .agent_processes
                .iter()
                .filter(|a| {
                    a.plan_id.as_deref() == Some(id.as_str())
                        || a.task_id
                            .as_deref()
                            .map(|tid| task_ids.contains(tid))
                            .unwrap_or(false)
                })
                .cloned()
                .collect(),
            Scope::Task { id, .. } => self
                .agent_processes
                .iter()
                .filter(|a| a.task_id.as_deref() == Some(id.as_str()))
                .cloned()
                .collect(),
            Scope::Agent { id } => self
                .agent_processes
                .iter()
                .filter(|a| a.id == *id)
                .cloned()
                .collect(),
            Scope::Pr { .. } => self
                .agent_processes
                .iter()
                .filter(|a| {
                    a.task_id
                        .as_deref()
                        .map(|tid| task_ids.contains(tid))
                        .unwrap_or(false)
                })
                .cloned()
                .collect(),
        }
    }

    /// PRs visible under the current scope.
    pub fn scoped_prs(&self) -> Vec<PrSummary> {
        match &self.scope {
            Scope::All => self.prs.clone(),
            Scope::Plan { id, .. } => {
                let task_ids = self
                    .tasks
                    .iter()
                    .filter(|t| t.plan_id == *id)
                    .map(|t| t.id.as_str())
                    .collect::<HashSet<_>>();
                self.prs
                    .iter()
                    .filter(|pr| {
                        pr.tracked_task_ids
                            .iter()
                            .any(|t| task_ids.contains(t.as_str()))
                    })
                    .cloned()
                    .collect()
            }
            Scope::Task { id, .. } => self
                .prs
                .iter()
                .filter(|pr| pr.tracked_task_ids.iter().any(|t| t == id))
                .cloned()
                .collect(),
            Scope::Agent { .. } => {
                let task_ids = self
                    .scoped_tasks()
                    .into_iter()
                    .map(|t| t.id)
                    .collect::<HashSet<_>>();
                self.prs
                    .iter()
                    .filter(|pr| pr.tracked_task_ids.iter().any(|t| task_ids.contains(t)))
                    .cloned()
                    .collect()
            }
            Scope::Pr { number, .. } => self
                .prs
                .iter()
                .filter(|pr| pr.number == *number)
                .cloned()
                .collect(),
        }
    }

    /// Bus messages visible under the current scope.
    pub fn scoped_messages(&self) -> Vec<BusMessage> {
        match &self.scope {
            Scope::All => self.messages.clone(),
            Scope::Plan { id, .. } | Scope::Task { plan_id: id, .. } => self
                .messages
                .iter()
                .filter(|m| m.plan_id.as_deref() == Some(id.as_str()))
                .cloned()
                .collect(),
            Scope::Agent { id } => self
                .messages
                .iter()
                .filter(|m| m.sender.as_deref() == Some(id.as_str()))
                .cloned()
                .collect(),
            Scope::Pr { .. } => Vec::new(),
        }
    }

    /// Explicit task links declared by one PR.
    pub fn pr_task_ids(&self, number: i64) -> HashSet<&str> {
        self.prs
            .iter()
            .find(|pr| pr.number == number)
            .map(|pr| pr.tracked_task_ids.iter().map(String::as_str).collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::{Plan, RegistryAgent, TaskSummary};

    fn plan(id: &str, title: &str) -> Plan {
        Plan {
            id: id.into(),
            title: title.into(),
            project: None,
            status: "active".into(),
            created_at: "2026-05-02".into(),
            updated_at: "2026-05-02".into(),
            ..Plan::default()
        }
    }

    fn task(id: &str, plan_id: &str, owner: Option<&str>) -> TaskSummary {
        TaskSummary {
            id: id.into(),
            plan_id: plan_id.into(),
            title: id.into(),
            status: "in_progress".into(),
            agent_id: owner.map(|s| s.into()),
            created_at: "2026-05-02".into(),
            updated_at: "2026-05-02".into(),
            ..TaskSummary::default()
        }
    }

    fn agent(id: &str) -> RegistryAgent {
        RegistryAgent {
            id: id.into(),
            kind: "claude".into(),
            status: Some("idle".into()),
            last_heartbeat_at: None,
            ..RegistryAgent::default()
        }
    }

    #[test]
    fn scoped_tasks_filter_only_after_plan_scope() {
        let mut s = AppState {
            plans: vec![plan("p1", "P1"), plan("p2", "P2")],
            tasks: vec![
                task("t1", "p1", None),
                task("t2", "p2", None),
                task("t3", "p1", None),
            ],
            ..AppState::default()
        };
        let all: Vec<String> = s.scoped_tasks().iter().map(|t| t.id.clone()).collect();
        assert_eq!(all, vec!["t1", "t2", "t3"]);
        s.scope = Scope::Plan {
            id: "p1".into(),
            title: "P1".into(),
        };
        let scoped: Vec<String> = s.scoped_tasks().iter().map(|t| t.id.clone()).collect();
        assert_eq!(scoped, vec!["t1", "t3"]);
    }

    #[test]
    fn scoped_agents_filters_to_owners_of_scoped_tasks() {
        let mut s = AppState {
            plans: vec![plan("p1", "P1"), plan("p2", "P2")],
            tasks: vec![
                task("t1", "p1", Some("alpha")),
                task("t2", "p2", Some("beta")),
            ],
            agents: vec![agent("alpha"), agent("beta"), agent("gamma")],
            ..AppState::default()
        };
        s.scope = Scope::Plan {
            id: "p1".into(),
            title: "P1".into(),
        };
        let scoped: Vec<&str> = s.scoped_agents().iter().map(|a| a.id.as_str()).collect();
        assert_eq!(scoped, vec!["alpha"]);
    }
}
