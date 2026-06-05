use super::util::{href, matches_ci, score_fields, upsert};
use super::SearchResult;
use crate::app::AppState;
use crate::error::ApiError;
use convergio_api::actions_registry;
use convergio_embed::semantic_search;
use std::collections::HashMap;

mod github_actions;

pub(super) async fn collect_structured(
    state: &AppState,
    query: &str,
    merged: &mut HashMap<(String, String), SearchResult>,
) -> Result<(), ApiError> {
    for plan in state.durability.plans().list(200).await? {
        if !matches_ci(
            query,
            [&plan.id, &plan.title, plan.project.as_deref().unwrap_or("")],
        ) {
            continue;
        }
        let score = score_fields(query, [&plan.id, &plan.title]);
        upsert(
            merged,
            SearchResult {
                kind: "plan".into(),
                id: plan.id.clone(),
                title: plan.title.clone(),
                subtitle: plan.project.clone(),
                href: href("plan", &plan.id),
                score: 70.0 + score,
                match_sources: vec!["structured".into()],
            },
        );
    }

    for task in state.durability.tasks().list(300).await? {
        let desc = task.description.clone().unwrap_or_default();
        if !matches_ci(query, [&task.id, &task.title, &desc, &task.plan_id]) {
            continue;
        }
        let score = score_fields(query, [&task.id, &task.title, &task.plan_id]);
        upsert(
            merged,
            SearchResult {
                kind: "task".into(),
                id: task.id.clone(),
                title: task.title.clone(),
                subtitle: Some(format!(
                    "plan:{} status:{}",
                    task.plan_id,
                    task.status.as_str()
                )),
                href: href("task", &task.id),
                score: 60.0 + score,
                match_sources: vec!["structured".into()],
            },
        );
    }

    for cap in state.durability.capabilities().list().await? {
        if !matches_ci(query, [&cap.name, &cap.version, &cap.status]) {
            continue;
        }
        let score = score_fields(query, [&cap.name]);
        upsert(
            merged,
            SearchResult {
                kind: "capability".into(),
                id: cap.name.clone(),
                title: cap.name.clone(),
                subtitle: Some(format!("{} ({})", cap.version, cap.status)),
                href: href("capability", &cap.name),
                score: 55.0 + score,
                match_sources: vec!["structured".into()],
            },
        );
    }

    for repo in state.fleet.list_repos().await? {
        if !matches_ci(query, [&repo.name, &repo.path, &repo.language, &repo.role]) {
            continue;
        }
        let score = score_fields(query, [&repo.name]);
        upsert(
            merged,
            SearchResult {
                kind: "repo".into(),
                id: repo.name.clone(),
                title: repo.name.clone(),
                subtitle: Some(repo.path.clone()),
                href: href("repo", &repo.name),
                score: 50.0 + score,
                match_sources: vec!["structured".into()],
            },
        );
    }

    for action in actions_registry() {
        if !matches_ci(query, [&action.name, action.capability, action.summary]) {
            continue;
        }
        let score = score_fields(query, [&action.name]);
        upsert(
            merged,
            SearchResult {
                kind: "action".into(),
                id: action.name.clone(),
                title: action.name.clone(),
                subtitle: Some(format!("{} — {}", action.capability, action.summary)),
                href: href("action", &action.name),
                score: 45.0 + score,
                match_sources: vec!["structured".into()],
            },
        );
    }

    Ok(())
}

pub(super) async fn collect_graph(
    state: &AppState,
    query: &str,
    merged: &mut HashMap<(String, String), SearchResult>,
) {
    match convergio_graph::search_nodes(&state.graph, query, 50).await {
        Ok(nodes) => {
            for n in nodes {
                let score = score_fields(query, [&n.id, &n.name]);
                let subtitle = match &n.file_path {
                    Some(p) => Some(format!("{} · {}", n.crate_name, p)),
                    None => Some(n.crate_name.clone()),
                };
                upsert(
                    merged,
                    SearchResult {
                        kind: "graph_node".into(),
                        id: n.id.clone(),
                        title: n.name.clone(),
                        subtitle,
                        href: href("graph_node", &n.id),
                        score: 40.0 + score,
                        match_sources: vec!["structured".into()],
                    },
                );
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, "graph search failed; continuing without graph hits");
        }
    }
}

pub(super) async fn collect_operational(
    state: &AppState,
    query: &str,
    merged: &mut HashMap<(String, String), SearchResult>,
) -> Result<(), ApiError> {
    for agent in state
        .durability
        .agents()
        .list_filtered(None, Some(200))
        .await?
    {
        if !matches_ci(
            query,
            [
                &agent.id,
                &agent.kind,
                agent.name.as_deref().unwrap_or(""),
                agent.host.as_deref().unwrap_or(""),
            ],
        ) {
            continue;
        }
        let score = score_fields(query, [&agent.id, &agent.kind]);
        upsert(
            merged,
            SearchResult {
                kind: "agent".into(),
                id: agent.id.clone(),
                title: agent.name.clone().unwrap_or_else(|| agent.id.clone()),
                subtitle: Some(format!("{} · {}", agent.kind, agent.status)),
                href: href("agent", &agent.id),
                score: 52.0 + score,
                match_sources: vec!["operational".into()],
            },
        );
    }

    for proc in state.supervisor.list(200).await? {
        let pid = proc.pid.map(|p| p.to_string()).unwrap_or_default();
        if !matches_ci(
            query,
            [
                &proc.id,
                &proc.kind,
                &proc.command,
                &pid,
                proc.task_id.as_deref().unwrap_or(""),
            ],
        ) {
            continue;
        }
        let score = score_fields(query, [&proc.id, &proc.command]);
        upsert(
            merged,
            SearchResult {
                kind: "process".into(),
                id: proc.id.clone(),
                title: proc.command.clone(),
                subtitle: Some(format!("{} pid:{}", proc.kind, pid)),
                href: href("process", &proc.id),
                score: 48.0 + score,
                match_sources: vec!["operational".into()],
            },
        );
    }

    github_actions::collect_github_actions(query, merged).await;

    Ok(())
}

pub(super) async fn collect_semantic(
    state: &AppState,
    query: &str,
    merged: &mut HashMap<(String, String), SearchResult>,
) -> Result<bool, ApiError> {
    match semantic_search(&state.embed, state.embedder.as_ref(), query, 25).await {
        Ok(neighbors) => {
            for n in neighbors {
                let score = (n.score as f64).clamp(0.0, 1.0);
                upsert(
                    merged,
                    SearchResult {
                        kind: "doc".into(),
                        id: n.node_id.clone(),
                        title: n.node_id.clone(),
                        subtitle: Some(n.repo.clone()),
                        href: href("doc", &n.node_id),
                        score: 30.0 + (score * 10.0),
                        match_sources: vec!["semantic".into()],
                    },
                );
            }
            Ok(false)
        }
        Err(convergio_embed::EmbedError::EmbedderFailed(msg)) => {
            tracing::warn!(error = %msg, "semantic search degraded; continuing with non-semantic hits");
            Ok(true)
        }
        Err(e) => Err(ApiError::Internal(format!("semantic search failed: {e}"))),
    }
}
