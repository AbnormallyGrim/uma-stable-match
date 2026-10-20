//! Finds candidate clubs and fetches their member data.

use chrono::{DateTime, Utc};
use uma_stable_match_umamoe_client::{
    Circle, CircleSort, Client, Error, ListCirclesRequest, Priority, SortDir,
};

use crate::details::ClubDetails;
use crate::pool::{game_month_start, jst, ClubPool, ListedClub};

// Safety stop in case the API never returns a short page.
const MAX_PAGES: u32 = 100;

/// Which clubs a scan looks at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanConfig {
    /// Best monthly rank to include.
    pub rank_from: u32,
    /// Worst monthly rank to include.
    pub rank_to: u32,
    /// Most clubs to fetch details for.
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
    pub prefetched: usize,
    /// Clubs whose details failed to load.
    pub failed: usize,
    pub month: Option<String>,
}

/// Why a scan failed.
#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    /// A list request failed.
    #[error("listing clubs failed: {0}")]
    List(#[source] Error),
}

impl ScanError {
    /// A short message that's safe to show visitors: no URLs or upstream error text.
    pub(crate) fn public_message(&self) -> String {
        match self {
            Self::List(error) => format!("listing clubs failed: {}", upstream_summary(error)),
        }
    }
}

fn upstream_summary(error: &Error) -> String {
    match error {
        Error::Status { status, .. } => format!("uma.moe returned {status}"),
        Error::Transport(_) => "could not reach uma.moe".into(),
        Error::Decode(_) => "unexpected response from uma.moe".into(),
        Error::Config(_) | Error::InvalidRequest(_) => "internal error".into(),
    }
}

/// Lists candidate clubs and fetches their member data.
pub async fn scan(
    client: &Client,
    details: &ClubDetails,
    config: &ScanConfig,
) -> Result<Scan, ScanError> {
    let rows = list_candidates(client, config).await?;
    let month = rows
        .iter()
        .filter_map(|c| c.last_updated.as_deref())
        .filter_map(|t| DateTime::parse_from_rfc3339(t).ok())
        .map(|t| t.with_timezone(&Utc))
        .max()
        .map(game_month);

    let mut pool = ClubPool::new(rows.iter().filter_map(ListedClub::from_circle));
    details.set_listed(rows.into_iter().map(|c| (c.circle_id, c)));
    details.restore_exact_medians(&mut pool);
    let candidates = pool.len();
    let ids = prefetch_ids(&pool, config.max_details as usize);
    details.replace_pool(pool);

    let mut prefetched = 0;
    let mut failed = 0;
    for circle_id in ids {
        match details.get(circle_id, Priority::Background).await {
            Ok(_) => prefetched += 1,
            Err(error) => {
                failed += 1;
                tracing::warn!(circle_id, %error, "skipping club that failed to load");
            }
        }
    }

    tracing::info!(candidates, prefetched, failed, "scan finished");
    Ok(Scan {
        candidates,
        prefetched,
        failed,
        month,
    })
}

fn prefetch_ids(pool: &ClubPool, max: usize) -> Vec<u64> {
    let ids: Vec<u64> = pool.clubs().map(|c| c.circle_id).collect();
    if ids.len() <= max {
        return ids;
    }
    (0..max).map(|i| ids[i * ids.len() / max]).collect()
}

fn game_month(at: DateTime<Utc>) -> String {
    game_month_start(at)
        .with_timezone(&jst())
        .format("%Y-%m")
        .to_string()
}

async fn list_candidates(client: &Client, config: &ScanConfig) -> Result<Vec<Circle>, ScanError> {
    let request = ListCirclesRequest {
        limit: Some(config.page_size),
        max_rank: Some(config.rank_to),
        sort_by: Some(CircleSort::MonthlyRank),
        sort_dir: Some(SortDir::Asc),
        ..Default::default()
    };

    let mut candidates = Vec::new();
    for page in 0..MAX_PAGES {
        let rows = client
            .list_circles(&request.page(page))
            .await
            .map_err(ScanError::List)?
            .circles;
        let Some(last) = rows.last() else { break };
        let done = last.monthly_rank.unwrap_or(0) >= config.rank_to as i32
            || rows.len() < config.page_size as usize;

        candidates.extend(rows.into_iter().filter(|c| in_window(c, config)));
        if done {
            break;
        }
    }
    Ok(candidates)
}

fn in_window(circle: &Circle, config: &ScanConfig) -> bool {
    circle
        .monthly_rank
        .and_then(|r| u32::try_from(r).ok())
        .is_some_and(|r| (config.rank_from..=config.rank_to).contains(&r))
}

#[cfg(test)]
mod tests;
