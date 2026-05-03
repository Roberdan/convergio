//! Prompt composition.
//!
//! Every runner sees the same prompt shape — the vendor only differs
//! in how it is delivered (`claude -p` text vs `copilot -p` text).
//! Keeping prompt assembly here means changing the agent contract is
//! one place to edit, not two.

use convergio_durability::Task;

/// Inputs the prompt builder needs.
pub struct PromptInputs<'a> {
    /// The task to be worked on.
    pub task: &'a Task,
    /// Plan id this task belongs to (already on `task` but cached
    /// here so callers without access to the durability handle can
    /// still build a prompt).
    pub plan_id: &'a str,
    /// Plan title (for the breadcrumb).
    pub plan_title: &'a str,
    /// Daemon base URL the agent will hit for `cvg evidence`,
    /// `cvg task transition`, etc. — usually `http://127.0.0.1:8420`.
    pub daemon_url: &'a str,
    /// Agent identity the runner will register under (also the value
    /// passed to `--agent-id` when the agent calls back).
    pub agent_id: &'a str,
    /// Optional context-pack from `convergio_graph::for_task_text`.
    /// Pass `None` when the graph is not built or when the task is
    /// new — the runner still has a usable prompt without it.
    pub graph_context: Option<&'a str>,
}

/// Build the full prompt.
///
/// The output is plain text; both `claude -p` and `copilot -p`
/// accept it as a single `<text>` argument. Length matters because
/// some terminals truncate long argv — the executor passes via
/// stdin instead (see `command.rs`).
pub fn build(inputs: &PromptInputs<'_>) -> String {
    let mut s = String::new();

    s.push_str(&format!(
        "You are a Convergio agent (id `{}`) working on a single task.\n",
        inputs.agent_id
    ));
    s.push_str(
        "Convergio is the leash: every state-change goes through `cvg` \
         (HTTP shell), the daemon owns the audit chain. Never bypass it.\n\n",
    );

    s.push_str("# Task\n\n");
    s.push_str(&format!("- id: `{}`\n", inputs.task.id));
    s.push_str(&format!(
        "- plan: `{}` — {}\n",
        inputs.plan_id, inputs.plan_title
    ));
    s.push_str(&format!("- title: {}\n", inputs.task.title));
    s.push_str(&format!(
        "- wave/sequence: {}/{}\n",
        inputs.task.wave, inputs.task.sequence
    ));
    if let Some(desc) = inputs.task.description.as_deref() {
        if !desc.is_empty() {
            s.push_str("- description:\n");
            for line in desc.lines() {
                s.push_str(&format!("  > {line}\n"));
            }
        }
    }
    if !inputs.task.evidence_required.is_empty() {
        s.push_str("- evidence required:\n");
        for kind in &inputs.task.evidence_required {
            s.push_str(&format!("  - `{kind}`\n"));
        }
    }
    s.push('\n');

    if let Some(ctx) = inputs.graph_context {
        if !ctx.trim().is_empty() {
            s.push_str("# Repo context (graph)\n\n");
            s.push_str(ctx.trim_end());
            s.push_str("\n\n");
        }
    }

    s.push_str("# Operating contract\n\n");
    s.push_str(&format!(
        "- Daemon URL: `{}`. All state changes via `cvg` against this URL.\n",
        inputs.daemon_url
    ));
    s.push_str("- Work in a fresh worktree under `.claude/worktrees/<branch>/`.\n");
    s.push_str("- Open exactly one PR. Body must include `Tracks: T<task_id>`.\n");
    s.push_str("- Attach evidence with `cvg evidence add <task_id> --kind <k> --payload '{...}'` for every required kind before transitioning.\n");
    s.push_str("- Move the task to `submitted` with `cvg task transition <task_id> submitted --agent-id <agent>`.\n");
    s.push_str("- Never push to `main`. Never bypass commit hooks. Never amend a public commit.\n");
    s.push_str("- If you cannot finish, leave the task in `in_progress` and stop. The reaper releases stale claims.\n");
    s.push_str("- Heartbeat every 60s with `cvg agent` (idempotent).\n");
    s.push('\n');
    s.push_str("Begin now.\n");

    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use convergio_durability::TaskStatus;

    fn fixture_task() -> Task {
        let now = Utc::now();
        Task {
            id: "t-1".into(),
            plan_id: "p-1".into(),
            wave: 1,
            sequence: 1,
            title: "Implement feature X".into(),
            description: Some("multi\nline detail".into()),
            status: TaskStatus::Pending,
            agent_id: None,
            evidence_required: vec!["test".into(), "code".into()],
            last_heartbeat_at: None,
            created_at: now,
            updated_at: now,
            started_at: None,
            ended_at: None,
            duration_ms: None,
            runner_kind: None,
            profile: None,
            max_budget_usd: None,
        }
    }

    #[test]
    fn prompt_includes_task_id_and_plan_breadcrumb() {
        let task = fixture_task();
        let p = build(&PromptInputs {
            task: &task,
            plan_id: "p-1",
            plan_title: "demo plan",
            daemon_url: "http://127.0.0.1:8420",
            agent_id: "claude-roberdan",
            graph_context: None,
        });
        assert!(p.contains("`t-1`"));
        assert!(p.contains("p-1"));
        assert!(p.contains("demo plan"));
        assert!(p.contains("Tracks: T<task_id>"));
    }

    #[test]
    fn prompt_includes_evidence_required_kinds() {
        let task = fixture_task();
        let p = build(&PromptInputs {
            task: &task,
            plan_id: "p-1",
            plan_title: "demo",
            daemon_url: "http://127.0.0.1:8420",
            agent_id: "claude",
            graph_context: None,
        });
        assert!(p.contains("`test`"));
        assert!(p.contains("`code`"));
    }

    #[test]
    fn prompt_includes_graph_context_when_present() {
        let task = fixture_task();
        let p = build(&PromptInputs {
            task: &task,
            plan_id: "p-1",
            plan_title: "demo",
            daemon_url: "http://127.0.0.1:8420",
            agent_id: "claude",
            graph_context: Some("file: src/foo.rs\nrelated: bar::baz\n"),
        });
        assert!(p.contains("# Repo context (graph)"));
        assert!(p.contains("src/foo.rs"));
    }

    #[test]
    fn prompt_skips_empty_graph_context() {
        let task = fixture_task();
        let p = build(&PromptInputs {
            task: &task,
            plan_id: "p-1",
            plan_title: "demo",
            daemon_url: "http://127.0.0.1:8420",
            agent_id: "claude",
            graph_context: Some("   "),
        });
        assert!(!p.contains("# Repo context"));
    }
}
