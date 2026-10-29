//! Runs scans on a schedule.

use std::future::Future;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::watch;
use uma_stable_match_umamoe_client::Circle;

use crate::cache::Cache;
use crate::cache::ScanPhase;
use crate::config::DEFAULT_SCAN_INTERVAL_MINUTES;
use crate::details::ClubDetails;
use crate::refresher::{utc_now, RefreshConfig};
use crate::scanner::{
    install, list_candidates, load, scan_period, scan_with, Scan, ScanConfig, ScanError,
};
use crate::shared::{ScanLease, ScanTurn, SharedCache};
use crate::source::Source;

const WAIT_POLL: Duration = Duration::from_secs(1);

#[derive(Serialize, Deserialize)]
struct PublishedScan {
    outcome: Result<Vec<Circle>, String>,
}

#[derive(Serialize, Deserialize)]
struct InProgress {
    listed_at_ms: u64,
    #[serde(default)]
    started_at_ms: u64,
    rows: Vec<Circle>,
}

impl InProgress {
    fn new(rows: Vec<Circle>, started: SystemTime, at: SystemTime) -> Self {
        Self {
            listed_at_ms: unix_millis(at),
            started_at_ms: unix_millis(started),
            rows,
        }
    }

    fn started(&self) -> SystemTime {
        let millis = match self.started_at_ms {
            0 => self.listed_at_ms,
            started => started,
        };
        UNIX_EPOCH + Duration::from_millis(millis)
    }

    fn is_current(&self, now: SystemTime, interval: Duration) -> bool {
        let listed = UNIX_EPOCH + Duration::from_millis(self.listed_at_ms);
        let Ok(age) = now.duration_since(listed) else {
            return false;
        };
        age < interval
            && scan_period(DateTime::<Utc>::from(listed)) == scan_period(DateTime::<Utc>::from(now))
    }
}

/// Scans now, then again until `shutdown` turns true: every `every` if set; with `None`, after
/// uma.moe's next daily update if `details` has a refresher, otherwise every
/// [`DEFAULT_SCAN_INTERVAL_MINUTES`](crate::DEFAULT_SCAN_INTERVAL_MINUTES).
///
/// With Redis, instances share one scan: one scans and publishes, the others load it. A published
/// scan stays current until the next scan is due. A scan in
/// progress at shutdown is cancelled.
pub async fn run_scheduler(
    source: impl Into<Source>,
    details: ClubDetails,
    config: ScanConfig,
    every: Option<Duration>,
    cache: Cache,
    mut shutdown: watch::Receiver<bool>,
) {
    let source: Source = source.into();
    let shared = details.shared().clone();
    let daily = every.is_none() && details.refreshes();
    let interval = every.unwrap_or(Duration::from_secs(
        u64::from(DEFAULT_SCAN_INTERVAL_MINUTES) * 60,
    ));
    let mut applied: Option<String> = None;
    let mut pooled: Option<String> = None;
    loop {
        let pause = match shared.scan_turn(&config).await {
            ScanTurn::Alone => {
                count_turn("alone");
                cache.begin_scan();
                tracing::info!("scan started");
                let started = SystemTime::now();
                cache.start_progress(ScanPhase::Listing, Some(started));
                let work = scan_with(&source, &details, &config, &cache);
                let Some(result) = until_stopped(work, &mut shutdown).await else {
                    return cancelled(&cache);
                };
                let updated_at = result.as_ref().ok().and_then(|scan| scan.updated_at);
                finish(&cache, result, "alone", started);
                if daily {
                    until_next_update(updated_at, utc_now())
                } else {
                    interval
                }
            }
            ScanTurn::Lead { lease, progress } => {
                count_turn("lead");
                cache.begin_scan();
                tracing::info!("scan started");
                applied = Some(lease.id().to_owned());
                let resumed = match progress {
                    Some(_) => resumable(&shared, &config, interval).await,
                    None => None,
                };
                let plan = match resumed {
                    Some(saved) => Plan::Resume(saved),
                    None => Plan::List {
                        started: SystemTime::now(),
                        keep_for: interval,
                    },
                };
                let started = plan.started();
                cache.start_progress(ScanPhase::Listing, Some(started));
                let work = lead(&source, &details, &config, &cache, &lease, plan);
                let Some(result) = until_stopped(work, &mut shutdown).await else {
                    lease.release().await;
                    return cancelled(&cache);
                };
                let (result, outcome) = match result {
                    Ok((scan, rows)) => (Ok(scan), Ok(rows)),
                    Err(error) => {
                        let message = error.public_message();
                        (Err(error), Err(message))
                    }
                };
                let fresh_for = if daily {
                    let updated_at = result.as_ref().ok().and_then(|scan| scan.updated_at);
                    until_next_update(updated_at, utc_now())
                } else {
                    interval
                };
                lease.publish(&PublishedScan { outcome }, fresh_for).await;
                finish(&cache, result, "lead", started);
                fresh_for
            }
            ScanTurn::Wait { progress } => {
                count_turn("wait");
                cache.begin_scan();
                if cache.status().progress.map(|p| p.phase) != Some(ScanPhase::Waiting) {
                    cache.start_progress(ScanPhase::Waiting, None);
                }
                if let Some(id) = progress.filter(|id| pooled.as_ref() != Some(id)) {
                    if let Some(saved) = resumable(&shared, &config, interval).await {
                        let started = saved.started();
                        let listed = saved.rows.len();
                        install(&details, saved.rows, &config);
                        details.restore_shared_medians().await;
                        cache.update_progress(|p| {
                            p.started_at = Some(started);
                            p.listed = listed;
                        });
                        tracing::info!("built the pool from the scan in progress");
                        pooled = Some(id);
                    }
                }
                WAIT_POLL
            }
            ScanTurn::Fresh { id, remaining } => {
                count_turn("fresh");
                if applied.as_deref() != Some(id.as_str()) {
                    cache.begin_scan();
                    let started = SystemTime::now();
                    cache.start_progress(ScanPhase::Loading, Some(started));
                    let work = sync(&source, &details, &config, &cache);
                    let Some(result) = until_stopped(work, &mut shutdown).await else {
                        return cancelled(&cache);
                    };
                    finish(&cache, result, "published", started);
                    applied = Some(id);
                }
                remaining
            }
        };

        tokio::select! {
            _ = tokio::time::sleep(pause) => {}
            _ = shutdown.wait_for(|stop| *stop) => return cache.cancel_scan(),
        }
    }
}

enum Plan {
    Resume(InProgress),
    List {
        started: SystemTime,
        keep_for: Duration,
    },
}

impl Plan {
    fn started(&self) -> SystemTime {
        match self {
            Self::Resume(saved) => saved.started(),
            Self::List { started, .. } => *started,
        }
    }
}

async fn lead(
    source: &Source,
    details: &ClubDetails,
    config: &ScanConfig,
    cache: &Cache,
    lease: &ScanLease,
    plan: Plan,
) -> Result<(Scan, Vec<Circle>), ScanError> {
    let work = async {
        let rows = match plan {
            Plan::Resume(saved) => {
                tracing::info!("resuming the scan in progress");
                saved.rows
            }
            Plan::List { started, keep_for } => {
                let rows = list_candidates(source, config, cache).await?;
                let progress = InProgress::new(rows.clone(), started, SystemTime::now());
                lease.save_progress(&progress, keep_for).await;
                rows
            }
        };
        Ok((load(details, rows.clone(), config, cache).await, rows))
    };
    tokio::pin!(work);
    loop {
        tokio::select! {
            result = &mut work => return result,
            _ = tokio::time::sleep(ScanLease::RENEW_EVERY) => lease.renew().await,
        }
    }
}

async fn sync(
    source: &Source,
    details: &ClubDetails,
    config: &ScanConfig,
    cache: &Cache,
) -> Result<Scan, ScanError> {
    match details.shared().published_scan(config).await {
        Some(PublishedScan { outcome: Ok(rows) }) => {
            tracing::info!("loading the scan published by another instance");
            Ok(load(details, rows, config, cache).await)
        }
        Some(PublishedScan {
            outcome: Err(message),
        }) => Err(ScanError::Shared(message)),
        None => {
            tracing::warn!("the published scan is missing; scanning here");
            cache.update_progress(|p| p.phase = ScanPhase::Listing);
            scan_with(source, details, config, cache).await
        }
    }
}

fn until_next_update(updated_at: Option<DateTime<Utc>>, now: DateTime<Utc>) -> Duration {
    RefreshConfig::default()
        .next_refresh(updated_at, now)
        .signed_duration_since(now)
        .to_std()
        .unwrap_or_default()
}

async fn until_stopped<T>(
    work: impl Future<Output = T>,
    shutdown: &mut watch::Receiver<bool>,
) -> Option<T> {
    tokio::select! {
        value = work => Some(value),
        _ = shutdown.wait_for(|stop| *stop) => None,
    }
}

fn finish(
    cache: &Cache,
    result: Result<Scan, ScanError>,
    source: &'static str,
    started: SystemTime,
) {
    let outcome = if result.is_ok() { "ok" } else { "error" };
    metrics::counter!("scans_total", "source" => source, "result" => outcome).increment(1);
    let took = SystemTime::now()
        .duration_since(started)
        .unwrap_or_default();
    metrics::histogram!("scan_duration_seconds", "source" => source).record(took.as_secs_f64());
    if let Err(error) = &result {
        tracing::error!(origin = error.origin(), %error, "scan failed; keeping the last results");
    }
    cache.finish_scan(result, SystemTime::now());
}

fn cancelled(cache: &Cache) {
    cache.cancel_scan();
    tracing::info!("scan cancelled for shutdown");
}

async fn resumable(
    shared: &SharedCache,
    config: &ScanConfig,
    interval: Duration,
) -> Option<InProgress> {
    let progress = shared.scan_in_progress::<InProgress>(config).await?;
    progress
        .is_current(SystemTime::now(), interval)
        .then_some(progress)
}

fn unix_millis(at: SystemTime) -> u64 {
    at.duration_since(UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

fn count_turn(turn: &'static str) {
    metrics::counter!("scan_turns_total", "turn" => turn).increment(1);
}

#[cfg(test)]
mod tests;
