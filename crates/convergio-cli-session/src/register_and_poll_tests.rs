//! Tests for [`crate::register_and_poll`].
//!
//! Hosted in a sibling file (`#[path = ...] mod tests`) so the main
//! module stays under the 300-line cap (CONSTITUTION § 13) and has
//! room for routine additions.

use super::*;
use std::sync::Mutex;

// Tests in a single binary share process-global env. Without
// serialization the `USER` / `CONVERGIO_AGENT_ID` writes race
// and the assertions are flaky.
static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn agent_id_uses_explicit_flag_first() {
    let _g = ENV_LOCK.lock().unwrap();
    std::env::set_var("CONVERGIO_AGENT_ID", "from-env");
    let id = resolve_agent_id(Some("from-flag".to_string()));
    assert_eq!(id, "from-flag");
    std::env::remove_var("CONVERGIO_AGENT_ID");
}

#[test]
fn agent_id_falls_back_to_user() {
    let _g = ENV_LOCK.lock().unwrap();
    std::env::remove_var("CONVERGIO_AGENT_ID");
    std::env::set_var("USER", "alice");
    let id = resolve_agent_id(None);
    assert_eq!(id, "claude-code-alice");
}

#[test]
fn uname_n_returns_non_empty() {
    let n = uname_n();
    assert!(!n.is_empty());
}
