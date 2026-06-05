//! Fleet-scoped dispatch helper.

use super::{dispatch::ExecutorMode, Client, OutputMode};
use anyhow::Result;
use serde_json::{json, Value};

/// Dispatch one executor tick scoped to a registered fleet repo.
pub async fn run(
    client: &Client,
    output: OutputMode,
    repo: &str,
    no_dispatch: bool,
    executor: ExecutorMode,
) -> Result<()> {
    let body: Value = client
        .post(
            "/v1/dispatch",
            &json!({
                "repo": repo,
                "no_dispatch": no_dispatch,
                "executor": executor.as_wire(),
            }),
        )
        .await?;
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&body)?),
        OutputMode::Plain => println!("{}", body["dispatched"].as_i64().unwrap_or(0)),
        OutputMode::Human => println!(
            "Fleet repo '{repo}' dispatch: {} task(s), executor={}",
            body["dispatched"].as_i64().unwrap_or(0),
            body["executor"].as_str().unwrap_or("default")
        ),
    }
    Ok(())
}
