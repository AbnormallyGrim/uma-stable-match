//! In-memory cache of scan results.

use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::SystemTime;

use crate::scanner::{Scan, ScanError};

/// Results of the last successful scan.
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    /// Clubs that passed the scan filters.
    pub candidates: usize,
    /// Clubs whose details were loaded.
    pub prefetched: usize,
    /// Clubs whose details failed to load.
    pub failed: usize,
    /// Game month of the listed data, as `YYYY-MM`.
    pub month: Option<String>,
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
    /// The running scan's progress, while a scan runs.
    pub progress: Option<ScanProgress>,
}

/// What a running scan is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanPhase {
    /// Listing clubs in the rank window.
    Listing,
    /// Loading club details.
    Loading,
    /// Waiting for another instance's scan, with Redis.
    Waiting,
}

/// How far the running scan has got.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanProgress {
    /// What the scan is doing.
    pub phase: ScanPhase,
    /// When the scan started. A resumed scan keeps its original start.
    pub started_at: Option<SystemTime>,
    /// Clubs in the rank window listed so far.
    pub listed: usize,
    /// Clubs whose details the scan loads.
    pub to_load: usize,
    /// Clubs loaded so far.
    pub loaded: usize,
    /// Clubs that failed to load.
    pub failed: usize,
}

impl ScanProgress {
    fn new(phase: ScanPhase, started_at: Option<SystemTime>) -> Self {
        Self {
            phase,
            started_at,
            listed: 0,
            to_load: 0,
            loaded: 0,
            failed: 0,
        }
    }
}

#[derive(Debug, Default)]
struct State {
    scanning: bool,
    snapshot: Option<Arc<Snapshot>>,
    last_error: Option<ScanFailure>,
    progress: Option<ScanProgress>,
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
            progress: state.progress.clone(),
        }
    }

    pub(crate) fn start_progress(&self, phase: ScanPhase, started_at: Option<SystemTime>) {
        let progress = ScanProgress::new(phase, started_at);
        record(Some(&progress));
        self.write().progress = Some(progress);
    }

    pub(crate) fn update_progress(&self, change: impl FnOnce(&mut ScanProgress)) {
        let mut state = self.write();
        if let Some(progress) = state.progress.as_mut() {
            change(progress);
            record(Some(progress));
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
        state.progress = None;
        record(None);
        match result {
            Ok(scan) => {
                state.snapshot = Some(Arc::new(Snapshot {
                    candidates: scan.candidates,
                    prefetched: scan.prefetched,
                    failed: scan.failed,
                    month: scan.month,
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
        let mut state = self.write();
        state.scanning = false;
        state.progress = None;
        record(None);
    }

    fn read(&self) -> RwLockReadGuard<'_, State> {
        self.state.read().unwrap_or_else(|e| e.into_inner())
    }

    fn write(&self) -> RwLockWriteGuard<'_, State> {
        self.state.write().unwrap_or_else(|e| e.into_inner())
    }
}

fn record(progress: Option<&ScanProgress>) {
    let phase = match progress.map(|p| p.phase) {
        Some(ScanPhase::Listing) => 1.0,
        Some(ScanPhase::Loading) => 2.0,
        Some(ScanPhase::Waiting) | None => 0.0,
    };
    metrics::gauge!("scan_phase").set(phase);
    if let Some(progress) = progress {
        metrics::gauge!("scan_listed_clubs").set(progress.listed as f64);
        metrics::gauge!("scan_clubs_to_load").set(progress.to_load as f64);
        metrics::gauge!("scan_clubs_loaded").set(progress.loaded as f64);
        metrics::gauge!("scan_clubs_failed").set(progress.failed as f64);
    }
}

#[cfg(test)]
mod tests;
