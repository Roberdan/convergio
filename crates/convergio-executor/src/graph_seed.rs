//! Compose the seed text the graph layer ranks task context
//! against. Concat title + description so both signals count.
//!
//! Pulled out of `executor.rs` so that module stays under the
//! 300-line cap.

use convergio_durability::Task;

pub(crate) fn build_graph_seed(task: &Task) -> String {
    match task.description.as_deref() {
        Some(d) if !d.is_empty() => format!("{}\n\n{}", task.title, d),
        _ => task.title.clone(),
    }
}
