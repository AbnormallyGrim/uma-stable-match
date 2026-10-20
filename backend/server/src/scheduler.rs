//! Runs scans on a schedule.

use std::time::{Duration, SystemTime};

use tokio::sync::watch;
use uma_stable_match_umamoe_client::Client;

use crate::cache::Cache;
use crate::details::ClubDetails;
use crate::scanner::{scan, ScanConfig};

/// Scans now, then every `interval`, until `shutdown` turns true.
///
/// A scan in progress at shutdown is cancelled.
pub async fn run_scheduler(
    client: Client,
    details: ClubDetails,
    config: ScanConfig,
    interval: Duration,
    cache: Cache,
    mut shutdown: watch::Receiver<bool>,
) {
    loop {
        if cache.begin_scan() {
            tracing::info!("scan started");
            tokio::select! {
                result = scan(&client, &details, &config) => {
                    if let Err(error) = &result {
                        tracing::error!(%error, "scan failed; keeping the last results");
                    }
                    cache.finish_scan(result, SystemTime::now());
                }
                _ = shutdown.wait_for(|stop| *stop) => {
                    cache.cancel_scan();
                    tracing::info!("scan cancelled for shutdown");
                    return;
                }
            }
        }

        tokio::select! {
            _ = tokio::time::sleep(interval) => {}
            _ = shutdown.wait_for(|stop| *stop) => return,
        }
    }
}
