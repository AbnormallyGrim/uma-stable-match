//! Runs scans on a schedule.

use std::future::Future;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::watch;
use uma_stable_match_umamoe_client::{Circle, Client};

use crate::cache::Cache;
use crate::details::ClubDetails;
use crate::scanner::{
    install, list_candidates, load, scan, scan_period, Scan, ScanConfig, ScanError, PHASE_IDLE,
};
use crate::shared::{ScanLease, ScanTurn, SharedCache};

const WAIT_POLL: Duration = Duration::from_secs(1);

#[derive(Serialize, Deserialize)]
struct PublishedScan {
    outcome: Result<Vec<Circle>, String>,
}

#[derive(Serialize, Deserialize)]
struct InProgress {
    listed_at_ms: u64,
    rows: Vec<Circle>,
}

impl InProgress {
    fn new(rows: Vec<Circle>, at: SystemTime) -> Self {
        Self {
            listed_at_ms: unix_millis(at),
            rows,
        }
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

/// Scans now, then every `interval`, until `shutdown` turns true.
///
/// With Redis, instances share one scan: one scans and publishes, the others load it. A scan in
/// progress at shutdown is cancelled.
pub async fn run_scheduler(
    client: Client,
    details: ClubDetails,
    config: ScanConfig,
    interval: Duration,
    cache: Cache,
    mut shutdown: watch::Receiver<bool>,
) {
    let shared = details.shared().clone();
    let mut applied: Option<String> = None;
    let mut pooled: Option<String> = None;
    loop {
        let pause = match shared.scan_turn(&config).await {
            ScanTurn::Alone => {
                count_turn("alone");
                cache.begin_scan();
                tracing::info!("scan started");
                let started = Instant::now();
                let work = scan(&client, &details, &config);
                let Some(result) = until_stopped(work, &mut shutdown).await else {
                    return cancelled(&cache);
                };
                finish(&cache, result, "alone", started);
                interval
            }
            ScanTurn::Lead { lease, progress } => {
                count_turn("lead");
                cache.begin_scan();
                tracing::info!("scan started");
                let started = Instant::now();
                applied = Some(lease.id().to_owned());
                let resumed = match progress {
                    Some(_) => resumable(&shared, &config, interval).await,
                    None => None,
                };
                let work = lead(&client, &details, &config, &lease, resumed, interval);
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
                lease.publish(&PublishedScan { outcome }, interval).await;
                finish(&cache, result, "lead", started);
                interval
            }
            ScanTurn::Wait { progress } => {
                count_turn("wait");
                cache.begin_scan();
                if let Some(id) = progress.filter(|id| pooled.as_ref() != Some(id)) {
                    if let Some(rows) = resumable(&shared, &config, interval).await {
                        install(&details, rows, &config);
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
                    let started = Instant::now();
                    let work = sync(&client, &details, &config);
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

async fn lead(
    client: &Client,
    details: &ClubDetails,
    config: &ScanConfig,
    lease: &ScanLease,
    resumed: Option<Vec<Circle>>,
    interval: Duration,
) -> Result<(Scan, Vec<Circle>), ScanError> {
    let work = async {
        let rows = match resumed {
            Some(rows) => {
                tracing::info!("resuming the scan in progress");
                rows
            }
            None => {
                let rows = list_candidates(client, config).await?;
                let progress = InProgress::new(rows.clone(), SystemTime::now());
                lease.save_progress(&progress, interval).await;
                rows
            }
        };
        Ok((load(details, rows.clone(), config).await, rows))
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
    client: &Client,
    details: &ClubDetails,
    config: &ScanConfig,
) -> Result<Scan, ScanError> {
    match details.shared().published_scan(config).await {
        Some(PublishedScan { outcome: Ok(rows) }) => {
            tracing::info!("loading the scan published by another instance");
            Ok(load(details, rows, config).await)
        }
        Some(PublishedScan {
            outcome: Err(message),
        }) => Err(ScanError::Shared(message)),
        None => {
            tracing::warn!("the published scan is missing; scanning here");
            scan(client, details, config).await
        }
    }
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

fn finish(cache: &Cache, result: Result<Scan, ScanError>, source: &'static str, started: Instant) {
    let outcome = if result.is_ok() { "ok" } else { "error" };
    metrics::counter!("scans_total", "source" => source, "result" => outcome).increment(1);
    metrics::histogram!("scan_duration_seconds", "source" => source)
        .record(started.elapsed().as_secs_f64());
    if let Err(error) = &result {
        tracing::error!(%error, "scan failed; keeping the last results");
    }
    cache.finish_scan(result, SystemTime::now());
}

fn cancelled(cache: &Cache) {
    metrics::gauge!("scan_phase").set(PHASE_IDLE);
    cache.cancel_scan();
    tracing::info!("scan cancelled for shutdown");
}

async fn resumable(
    shared: &SharedCache,
    config: &ScanConfig,
    interval: Duration,
) -> Option<Vec<Circle>> {
    let progress = shared.scan_in_progress::<InProgress>(config).await?;
    progress
        .is_current(SystemTime::now(), interval)
        .then_some(progress.rows)
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
