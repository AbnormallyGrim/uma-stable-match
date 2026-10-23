//! Finds candidate clubs and fetches their member data.

use std::collections::HashSet;
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use uma_stable_match_umamoe_client::{
    Circle, CircleSort, Client, Error, ListCirclesRequest, Priority, SortDir,
};

use crate::cache::{Cache, ScanPhase};
use crate::details::ClubDetails;
use crate::pool::{game_month, is_early_month, previous_game_month, ClubPool, ListedClub, Period};

/// Which clubs a scan looks at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanConfig {
    /// Best monthly rank to include.
    pub rank_from: u32,
    /// Worst monthly rank to include.
    pub rank_to: u32,
    /// Most clubs to keep details for, best rank first.
    pub max_details: u32,
    /// Clubs per list page.
    pub page_size: u32,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            rank_from: 80,
            rank_to: 900,
            max_details: 200,
            page_size: 100,
        }
    }
}

/// Result of one scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scan {
    /// Clubs that passed the scan filters.
    pub candidates: usize,
    /// Clubs whose details were loaded.
    pub prefetched: usize,
    /// Clubs whose details failed to load.
    pub failed: usize,
    /// Game month of the listed data, as `YYYY-MM`.
    pub month: Option<String>,
    /// When uma.moe last updated the listed data.
    pub updated_at: Option<DateTime<Utc>>,
}

/// Why a scan failed.
#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    /// A list request failed.
    #[error("listing clubs failed: {0}")]
    List(#[source] Error),
    /// Another instance's scan failed, with its public message.
    #[error("{0}")]
    Shared(String),
}

impl ScanError {
    /// Where the error came from, as [`Error::origin`]. A scan published by another instance counts
    /// as `"server"`.
    pub(crate) fn origin(&self) -> &'static str {
        match self {
            Self::List(error) => error.origin(),
            Self::Shared(_) => "server",
        }
    }

    /// A short message that's safe to show visitors: no URLs or upstream error text.
    pub(crate) fn public_message(&self) -> String {
        match self {
            Self::List(error) => format!("listing clubs failed: {}", upstream_summary(error)),
            Self::Shared(message) => message.clone(),
        }
    }
}

fn upstream_summary(error: &Error) -> String {
    match error {
        Error::Status { status, .. } => format!("uma.moe returned {status}"),
        Error::RateLimited { .. } => "uma.moe asked to slow down".into(),
        Error::Transport(_) => "could not reach uma.moe".into(),
        Error::Decode(_) => "unexpected response from uma.moe".into(),
        Error::Config(_) | Error::InvalidRequest(_) => "internal error".into(),
    }
}

/// Lists the rank window into a new club pool and swaps it into `details`. Then, if `details` has a
/// refresher, schedules the clubs for it; otherwise prefetches club details in the background lane.
pub async fn scan(
    client: &Client,
    details: &ClubDetails,
    config: &ScanConfig,
) -> Result<Scan, ScanError> {
    scan_with(client, details, config, &Cache::new()).await
}

pub(crate) async fn scan_with(
    client: &Client,
    details: &ClubDetails,
    config: &ScanConfig,
    cache: &Cache,
) -> Result<Scan, ScanError> {
    let rows = list_candidates(client, config, cache).await?;
    Ok(load(details, rows, config, cache).await)
}

pub(crate) async fn load(
    details: &ClubDetails,
    rows: Vec<Circle>,
    config: &ScanConfig,
    cache: &Cache,
) -> Scan {
    let listed = rows.len();
    let Installed {
        candidates,
        ids,
        month,
        updated_at,
    } = install(details, rows, config);
    if details.refreshes() {
        details.schedule(ids).await;
        let prefetched = details.cached_in_pool().len();
        tracing::info!(candidates, prefetched, "scan listed the clubs");
        return Scan {
            candidates,
            prefetched,
            failed: 0,
            month,
            updated_at,
        };
    }

    cache.update_progress(|p| {
        p.phase = ScanPhase::Loading;
        p.listed = listed;
        p.to_load = ids.len();
        p.loaded = 0;
        p.failed = 0;
    });
    let mut prefetched = 0;
    let mut failed = 0;
    for (circle_id, _) in ids {
        match details.get(circle_id, Priority::Background).await {
            Ok(_) => {
                prefetched += 1;
                cache.update_progress(|p| p.loaded = prefetched);
            }
            Err(error) => {
                failed += 1;
                cache.update_progress(|p| p.failed = failed);
                tracing::warn!(circle_id, origin = error.origin(), %error, "skipping club that failed to load");
            }
        }
    }

    tracing::info!(candidates, prefetched, failed, "scan finished");
    Scan {
        candidates,
        prefetched,
        failed,
        month,
        updated_at,
    }
}

pub(crate) struct Installed {
    candidates: usize,
    ids: Vec<(u64, u32)>,
    month: Option<String>,
    updated_at: Option<DateTime<Utc>>,
}

pub(crate) fn install(details: &ClubDetails, rows: Vec<Circle>, config: &ScanConfig) -> Installed {
    let snapshot = rows
        .iter()
        .filter_map(|c| c.last_updated.as_deref())
        .filter_map(|t| DateTime::parse_from_rfc3339(t).ok())
        .map(|t| t.with_timezone(&Utc))
        .max();
    let period = snapshot
        .filter(|at| is_early_month(*at))
        .map(previous_game_month);
    let month = snapshot.map(|at| month_label(period.unwrap_or_else(|| game_month(at))));

    let listed = if period.is_some() {
        ListedClub::from_circle_last_month
    } else {
        ListedClub::from_circle
    };
    let mut pool = ClubPool::new(rows.iter().filter_map(listed));
    let ids = prefetch_ids(&rows, &pool, config.max_details as usize);
    details.set_period(period);
    details.set_listed(rows.into_iter().map(|c| (c.circle_id, c)));
    details.restore_exact_medians(&mut pool);
    let candidates = pool.len();
    details.replace_pool(pool);
    details.set_listing(SystemTime::now(), month.clone());
    Installed {
        candidates,
        ids,
        month,
        updated_at: snapshot,
    }
}

pub(crate) fn scan_period(at: DateTime<Utc>) -> Period {
    if is_early_month(at) {
        previous_game_month(at)
    } else {
        game_month(at)
    }
}

fn prefetch_ids(rows: &[Circle], pool: &ClubPool, max: usize) -> Vec<(u64, u32)> {
    let mut ranked: Vec<(u64, u32)> = rows
        .iter()
        .filter(|c| pool.contains(c.circle_id))
        .map(|c| (c.circle_id, rank(c)))
        .collect();
    ranked.sort_by_key(|&(id, rank)| (rank, id));
    let mut seen = HashSet::new();
    ranked.retain(|&(id, _)| seen.insert(id));
    ranked.truncate(max);
    ranked
}

fn rank(circle: &Circle) -> u32 {
    circle
        .monthly_rank
        .and_then(|r| u32::try_from(r).ok())
        .unwrap_or(u32::MAX)
}

fn month_label((year, month): Period) -> String {
    format!("{year:04}-{month:02}")
}

pub(crate) async fn list_candidates(
    client: &Client,
    config: &ScanConfig,
    cache: &Cache,
) -> Result<Vec<Circle>, ScanError> {
    let request = ListCirclesRequest {
        limit: Some(config.page_size),
        max_rank: Some(config.rank_to),
        sort_by: Some(CircleSort::MonthlyRank),
        sort_dir: Some(SortDir::Asc),
        ..Default::default()
    };

    let mut candidates = Vec::new();
    for page in 0..max_pages(config) {
        let rows = client
            .list_circles(&request.page(page))
            .await
            .map_err(ScanError::List)?
            .circles;
        let Some(last) = rows.last() else { break };
        let done = last.monthly_rank.unwrap_or(0) >= config.rank_to as i32
            || rows.len() < config.page_size as usize;

        candidates.extend(rows.into_iter().filter(|c| in_window(c, config)));
        cache.update_progress(|p| p.listed = candidates.len());
        if done {
            break;
        }
    }
    Ok(candidates)
}

// Safety stop in case the API never returns a short page: enough pages to reach `rank_to`, plus
// one.
fn max_pages(config: &ScanConfig) -> u32 {
    config
        .rank_to
        .div_ceil(config.page_size.max(1))
        .saturating_add(1)
}

fn in_window(circle: &Circle, config: &ScanConfig) -> bool {
    circle
        .monthly_rank
        .and_then(|r| u32::try_from(r).ok())
        .is_some_and(|r| (config.rank_from..=config.rank_to).contains(&r))
}

#[cfg(test)]
mod tests;
