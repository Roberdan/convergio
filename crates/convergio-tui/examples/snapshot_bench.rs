//! Quick perf check for `Client::snapshot()`. Run against a live
//! daemon: `cargo run -p convergio-tui --release --example snapshot_bench`.

use convergio_tui::client::Client;
use std::time::Instant;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let base =
        std::env::var("CONVERGIO_URL").unwrap_or_else(|_| "http://127.0.0.1:8420".to_owned());
    // Skip gh pr list — exterior shell-out, not what we measure.
    std::env::set_var("CONVERGIO_DASH_NO_GH", "1");
    let client = Client::new(base);

    // Warm-up so connection pool is established.
    let _ = client.snapshot().await?;

    let runs = 5;
    let mut samples = Vec::with_capacity(runs);
    for _ in 0..runs {
        let t = Instant::now();
        let snap = client.snapshot().await?;
        let elapsed = t.elapsed();
        samples.push(elapsed);
        println!(
            "snapshot {:>6.1?}  plans={} tasks={} agents={} processes={} messages={}",
            elapsed,
            snap.plans.len(),
            snap.tasks.len(),
            snap.agents.len(),
            snap.agent_processes.len(),
            snap.messages.len(),
        );
    }
    samples.sort();
    let total: u128 = samples.iter().map(|d| d.as_micros()).sum();
    println!(
        "\nmin {:>6.1?}  median {:>6.1?}  max {:>6.1?}  avg {:>6.1?}",
        samples[0],
        samples[runs / 2],
        samples[runs - 1],
        std::time::Duration::from_micros((total / runs as u128) as u64),
    );
    Ok(())
}
