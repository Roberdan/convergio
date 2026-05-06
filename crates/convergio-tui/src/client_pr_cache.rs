//! Time-bounded memo for `gh pr list` results.
//!
//! Split out from [`crate::client`] so that file stays under the
//! 300-line cap and the locking pattern is testable in isolation.

use crate::client::PrSummary;
use anyhow::Result;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// PR data is cached for this long before the next `gh pr list`
/// shell-out. The dashboard tick is 5s by default; 30s means the
/// `gh` cost is amortised across ~6 refreshes without making the
/// PR pane feel stale (PR state turns over much slower than tasks).
pub const PR_CACHE_TTL: Duration = Duration::from_secs(30);

/// Time-stamped cache entry for one slice of the PR list.
pub type PrCacheCell = Arc<Mutex<Option<(Instant, Vec<PrSummary>)>>>;

/// Read the current cache entry, if any.
pub fn read_slot(slot: &PrCacheCell) -> Option<(Instant, Vec<PrSummary>)> {
    let guard = match slot.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    guard.clone()
}

/// Replace the cache entry with `prs` stamped at the current
/// instant.
pub fn write_slot(slot: &PrCacheCell, prs: Vec<PrSummary>) {
    let mut guard = match slot.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    *guard = Some((Instant::now(), prs));
}

/// Read-through cache: return the cached vector when fresh,
/// otherwise await `fetch`, store its result on success, and fall
/// back to the previous (possibly stale) cache on failure. Returns
/// an empty vector when `enable_gh` is `false`.
pub async fn cached_fetch(
    enable_gh: bool,
    slot: &PrCacheCell,
    fetch: impl std::future::Future<Output = Result<Vec<PrSummary>>>,
) -> Vec<PrSummary> {
    if !enable_gh {
        return Vec::new();
    }
    if let Some((stamped_at, cached)) = read_slot(slot) {
        if stamped_at.elapsed() < PR_CACHE_TTL {
            return cached;
        }
    }
    match fetch.await {
        Ok(prs) => {
            write_slot(slot, prs.clone());
            prs
        }
        Err(_) => read_slot(slot).map(|(_, v)| v).unwrap_or_default(),
    }
}
