#![allow(missing_docs)]

use convergio_db::Pool;
use convergio_durability::{init, Durability};
use convergio_executor::{Executor, SpawnTemplate};
use convergio_lifecycle::Supervisor;
use tempfile::tempdir;

pub async fn fresh_with(template: SpawnTemplate) -> (Executor, Durability, tempfile::TempDir) {
    // Tests should not depend on operator env; this flag forces the
    // legacy SpawnTemplate seam unless the task itself carries runner_kind.
    std::env::remove_var("CONVERGIO_EXECUTOR_USE_RUNNER");
    std::env::remove_var("CONVERGIO_EXECUTOR_MAX_PARALLEL");

    let dir = tempdir().unwrap();
    let url = format!("sqlite://{}/state.db", dir.path().display());
    let pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    convergio_lifecycle::init(&pool).await.unwrap();
    let dur = Durability::new(pool.clone());
    let sup = Supervisor::new(pool);
    let exec = Executor::new(dur.clone(), sup, template);
    (exec, dur, dir)
}

pub async fn fresh() -> (Executor, Durability, tempfile::TempDir) {
    fresh_with(SpawnTemplate::default()).await
}
