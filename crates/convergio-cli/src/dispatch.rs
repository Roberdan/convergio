//! Subcommand dispatch — kept out of `main.rs` so that file stays
//! under the 300-line cap when new top-level commands or pre-dispatch
//! hooks (e.g. drift warning) land. Pure routing logic only.

use crate::commands;
use crate::Command;
use anyhow::Result;
use convergio_i18n::Bundle;

/// Route the parsed `cmd` to the matching subcommand runner.
pub(crate) async fn run(
    client: commands::Client,
    bundle: Bundle,
    output: commands::OutputMode,
    cmd: Command,
) -> Result<()> {
    match cmd {
        Command::Health => commands::health::run(&client, &bundle, output).await,
        Command::Setup { sub } => commands::setup::run(&client, &bundle, output, sub).await,
        Command::Doctor { json, kill_zombies } => {
            commands::doctor::run(&client, &bundle, output, json, kill_zombies).await
        }
        Command::Status {
            completed_limit,
            project,
            all,
            show_waves,
            mine,
        } => {
            commands::status::run(
                &client,
                &bundle,
                output,
                completed_limit,
                project,
                all,
                show_waves,
                mine,
            )
            .await
        }
        Command::Plan { sub } => commands::plan::run(&client, &bundle, output, sub).await,
        Command::Task { sub } => commands::task::run(&client, output, sub).await,
        Command::Evidence { sub } => commands::evidence::run(&client, sub).await,
        Command::Audit { sub } => commands::audit::run(&client, sub).await,
        Command::Agent { sub } => commands::agent::run(&client, &bundle, output, sub).await,
        Command::Crdt { sub } => commands::crdt::run(&client, &bundle, output, sub).await,
        Command::Capability { sub } => {
            commands::capability::run(&client, &bundle, output, sub).await
        }
        Command::Coherence { sub } => commands::coherence::run(&bundle, output, sub).await,
        Command::Docs { sub } => commands::docs::run(output, sub).await,
        Command::Graph { sub } => commands::graph::run(&client, output, sub).await,
        Command::Embed { sub } => commands::embed::run(&client, output, sub).await,
        Command::Fleet { sub } => commands::fleet::run(&client, output, sub).await,
        Command::Workspace { sub } => commands::workspace::run(&client, &bundle, output, sub).await,
        Command::Mcp { sub } => commands::mcp::run(&bundle, sub).await,
        Command::Pr { sub } => commands::pr::run(&client, &bundle, output, sub).await,
        Command::Service { sub } => commands::service::run(&bundle, sub).await,
        Command::Session { sub } => commands::session::run(&client, &bundle, output, sub).await,
        Command::Solve { mission } => commands::solve::run(&client, &mission).await,
        Command::Dispatch => commands::dispatch::run(&client).await,
        Command::Validate {
            plan_id,
            wave,
            self_test,
        } => commands::validate::run(&client, plan_id.as_deref(), wave, self_test).await,
        Command::About { animate } => commands::about::run(&bundle, animate),
        Command::Monitor { tick_secs } => commands::monitor::run(&client, tick_secs).await,
        Command::Demo => commands::demo::run(&client).await,
        Command::Dash { tick_secs } => commands::dash::run(client.base(), tick_secs).await,
        Command::Update {
            if_needed,
            skip_restart,
            changelog,
        } => {
            commands::update::run(&client, &bundle, output, if_needed, skip_restart, changelog)
                .await
        }
        Command::Bus { sub } => commands::bus::run(&client, &bundle, output, sub).await,
        Command::Discover { since, agent_id } => {
            let args = commands::discover::DiscoverArgs { since, agent_id };
            commands::discover::run(&client, &bundle, output, args).await
        }
    }
}
