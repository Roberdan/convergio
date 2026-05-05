//! `convergio-mcp` — stdio MCP bridge for the local daemon.

mod actions;
#[cfg(test)]
mod actions_tests;
mod bridge;
mod bus_actions;
#[cfg(test)]
mod e2e_tests;
mod help;
mod http;

use anyhow::Result;
use bridge::Bridge;
use clap::Parser;
use rmcp::service::ServiceExt;

#[derive(Parser)]
#[command(name = "convergio-mcp", version, about = "Convergio MCP bridge")]
struct Cli {
    /// Local daemon base URL.
    #[arg(long, env = "CONVERGIO_URL", default_value = "http://127.0.0.1:8420")]
    url: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let service = Bridge::new(cli.url).serve(rmcp::transport::stdio()).await?;
    service.waiting().await?;
    Ok(())
}
