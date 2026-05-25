//! `cvg plan-templates …` — list, show and render plan templates
//! (W6, ADR-0056).
//!
//! Templates are first-party scaffolds living in
//! `convergio-planner::templates`. This command surfaces them as a
//! discoverable CLI verb and lets the operator render one into a JSON
//! plan shape ready to feed into `cvg plan create` + `cvg task create`
//! (or a future `cvg plan create --template` shortcut).

use anyhow::{anyhow, Result};
use clap::Subcommand;
use convergio_planner::templates::{get_builtin, list_builtin};
use std::collections::HashMap;

use super::OutputMode;

/// `cvg plan-templates` subcommands.
#[derive(Subcommand)]
pub enum PlanTemplatesCommand {
    /// List every built-in template.
    List,
    /// Show metadata + required parameters for one template.
    Show {
        /// Template name, e.g. `vertical-accelerator-v1`.
        name: String,
    },
    /// Render a template with the supplied parameters and print the
    /// resulting plan shape as JSON on stdout.
    Render {
        /// Template name.
        name: String,
        /// Parameter binding, repeatable. Format: `key=value`.
        #[arg(long = "param", value_name = "KEY=VALUE")]
        params: Vec<String>,
    },
}

/// Dispatch.
pub async fn run(output: OutputMode, cmd: PlanTemplatesCommand) -> Result<()> {
    match cmd {
        PlanTemplatesCommand::List => render_list(output),
        PlanTemplatesCommand::Show { name } => render_show(output, &name),
        PlanTemplatesCommand::Render { name, params } => render_render(output, &name, params),
    }
}

fn render_list(output: OutputMode) -> Result<()> {
    let items: Vec<_> = list_builtin()
        .iter()
        .map(|t| {
            serde_json::json!({
                "name": t.name,
                "summary": t.summary,
                "parameters": t.parameters.iter().map(|p| p.name).collect::<Vec<_>>(),
                "tasks": t.tasks.len(),
            })
        })
        .collect();
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&items)?),
        OutputMode::Plain => {
            for t in list_builtin() {
                println!("{}", t.name);
            }
        }
        OutputMode::Human => {
            for t in list_builtin() {
                let params: Vec<_> = t.parameters.iter().map(|p| p.name).collect();
                println!("- {} ({} task(s))", t.name, t.tasks.len());
                println!("    {}", t.summary);
                println!("    params: {}", params.join(", "));
            }
        }
    }
    Ok(())
}

fn render_show(output: OutputMode, name: &str) -> Result<()> {
    let t = get_builtin(name).ok_or_else(|| anyhow!("unknown template: {name}"))?;
    let body = serde_json::json!({
        "name": t.name,
        "summary": t.summary,
        "description": t.description,
        "objective": t.objective,
        "title": t.title,
        "parameters": t.parameters.iter().map(|p| serde_json::json!({
            "name": p.name,
            "help": p.help,
        })).collect::<Vec<_>>(),
        "tasks": t.tasks.iter().map(|tk| serde_json::json!({
            "wave": tk.wave,
            "sequence": tk.sequence,
            "title": tk.title,
            "description": tk.description,
            "evidence_required": tk.evidence_required,
        })).collect::<Vec<_>>(),
    });
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&body)?),
        OutputMode::Plain | OutputMode::Human => {
            println!("template: {}", t.name);
            println!("summary:  {}", t.summary);
            if let Some(d) = t.description {
                println!("description: {d}");
            }
            println!("objective:   {}", t.objective);
            println!("title:       {}", t.title);
            println!("parameters:");
            for p in t.parameters {
                println!("  - {}: {}", p.name, p.help);
            }
            println!("tasks ({}):", t.tasks.len());
            for tk in t.tasks {
                println!("  - w{} s{} {}", tk.wave, tk.sequence, tk.title);
            }
        }
    }
    Ok(())
}

fn render_render(output: OutputMode, name: &str, raw_params: Vec<String>) -> Result<()> {
    let t = get_builtin(name).ok_or_else(|| anyhow!("unknown template: {name}"))?;
    let mut params = HashMap::with_capacity(raw_params.len());
    for kv in &raw_params {
        let (k, v) = kv
            .split_once('=')
            .ok_or_else(|| anyhow!("expected key=value, got `{kv}`"))?;
        params.insert(k.trim().to_string(), v.trim().to_string());
    }
    let rendered = t
        .render(&params)
        .map_err(|e| anyhow!("render failed: {e}"))?;
    let body = serde_json::json!({
        "objective": rendered.objective,
        "plan": {
            "title": rendered.plan.title,
            "description": rendered.plan.description,
            "tasks": rendered.plan.tasks,
        },
    });
    match output {
        OutputMode::Json | OutputMode::Human => {
            println!("{}", serde_json::to_string_pretty(&body)?);
        }
        OutputMode::Plain => {
            println!("{}", serde_json::to_string(&body)?);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn render_known_template_succeeds() {
        let cmd = PlanTemplatesCommand::Render {
            name: "vertical-accelerator-v1".into(),
            params: vec![
                "domain=education".into(),
                "primary_language=en".into(),
                "secondary_language=it".into(),
                "target_audience=K-12 teachers".into(),
            ],
        };
        run(OutputMode::Json, cmd).await.unwrap();
    }

    #[tokio::test]
    async fn unknown_template_errors() {
        let cmd = PlanTemplatesCommand::Show {
            name: "does-not-exist".into(),
        };
        let err = run(OutputMode::Json, cmd).await.unwrap_err();
        assert!(format!("{err}").contains("unknown template"));
    }

    #[tokio::test]
    async fn malformed_param_errors() {
        let cmd = PlanTemplatesCommand::Render {
            name: "vertical-accelerator-v1".into(),
            params: vec!["no_equals_sign".into()],
        };
        let err = run(OutputMode::Json, cmd).await.unwrap_err();
        assert!(format!("{err}").contains("expected key=value"));
    }
}
