use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::SystemTime;

use uma_stable_match_scoring::Club;

use crate::scanner::{Scan, ScanError};

#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub clubs: Vec<Club>,
    pub candidates: usize,
    pub failed: usize,
    pub scanned_at: SystemTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanFailure {
    pub at: SystemTime,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CacheStatus {
    pub scanning: bool,
    pub snapshot: Option<Arc<Snapshot>>,
    pub last_error: Option<ScanFailure>,
}

#[derive(Debug, Default)]
struct State {
    scanning: bool,
    snapshot: Option<Arc<Snapshot>>,
    last_error: Option<ScanFailure>,
}

#[derive(Debug, Clone, Default)]
pub struct Cache {
    state: Arc<RwLock<State>>,
}

impl Cache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> Option<Arc<Snapshot>> {
        self.read().snapshot.clone()
    }

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
                    message: error.to_string(),
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
