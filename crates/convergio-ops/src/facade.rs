use crate::store::{OpsWorkflowInstanceStore, OpsWorkflowStore};
use convergio_db::Pool;

mod helpers;
mod instances;
mod workflows;

/// Ops workflow engine facade.
#[derive(Clone)]
pub struct Ops {
    pub(crate) pool: Pool,
}

impl Ops {
    /// Wrap a pool.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Workflow store accessor.
    pub fn workflows(&self) -> OpsWorkflowStore {
        OpsWorkflowStore::new(self.pool.clone())
    }

    /// Instance store accessor.
    pub fn instances(&self) -> OpsWorkflowInstanceStore {
        OpsWorkflowInstanceStore::new(self.pool.clone())
    }
}
