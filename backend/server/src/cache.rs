//! In-memory cache of scan results.

use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::SystemTime;

use uma_stable_match_scoring::Club;

use crate::scanner::{Scan, ScanError};

/// Results of the last successful scan.
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    /// Clubs with member data, climbers first.
    pub clubs: Vec<Club>,
    /// Clubs that matched the rank window and had open spots.
    pub candidates: usize,
    /// Clubs whose details failed to load.
    pub failed: usize,
    /// When the scan finished.
    pub scanned_at: SystemTime,
}

/// The most recent failed scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanFailure {
    /// When the scan failed.
    pub at: SystemTime,
    /// Why it failed.
    pub message: String,
}

/// What the cache holds right now.
#[derive(Debug, Clone, PartialEq)]
pub struct CacheStatus {
    /// Whether a scan is running.
    pub scanning: bool,
    /// Last successful scan, if any.
    pub snapshot: Option<Arc<Snapshot>>,
    /// Failure since the last successful scan, if any.
    pub last_error: Option<ScanFailure>,
}

#[derive(Debug, Default)]
struct State {
    scanning: bool,
    snapshot: Option<Arc<Snapshot>>,
    last_error: Option<ScanFailure>,
}

/// Shared in-memory scan results. Clones share the same state.
#[derive(Debug, Clone, Default)]
pub struct Cache {
    state: Arc<RwLock<State>>,
}

impl Cache {
    /// An empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Last successful scan, if any.
    pub fn snapshot(&self) -> Option<Arc<Snapshot>> {
        self.read().snapshot.clone()
    }

    /// Scan state, last results and last failure.
    pub fn status(&self) -> CacheStatus {
        let state = self.read();
        CacheStatus {
            scanning: state.scanning,
            snapshot: state.snapshot.clone(),
            last_error: state.last_error.clone(),
        }
    }

    pub(crate) fn begin_scan(&self) -> bool {
        let mut state = self.write();
        if state.scanning {
            return false;
        }
        state.scanning = true;
        true
    }

    pub(crate) fn finish_scan(&self, result: Result<Scan, ScanError>, at: SystemTime) {
        let mut state = self.write();
        state.scanning = false;
        match result {
            Ok(scan) => {
                state.snapshot = Some(Arc::new(Snapshot {
                    clubs: scan.clubs,
                    candidates: scan.candidates,
                    failed: scan.failed,
                    scanned_at: at,
                }));
                state.last_error = None;
            }
            Err(error) => {
                state.last_error = Some(ScanFailure {
                    at,
                    message: error.public_message(),
                });
            }
        }
    }

    pub(crate) fn cancel_scan(&self) {
        self.write().scanning = false;
    }

    fn read(&self) -> RwLockReadGuard<'_, State> {
        self.state.read().unwrap_or_else(|e| e.into_inner())
    }

    fn write(&self) -> RwLockWriteGuard<'_, State> {
        self.state.write().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
mod tests;
