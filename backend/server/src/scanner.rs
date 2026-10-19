//! Finds candidate clubs and fetches their member data.

use std::cmp::Reverse;

use uma_stable_match_scoring::{Club, CLUB_CAP};
use uma_stable_match_umamoe_client::{
    Circle, CircleRequest, CircleSort, Client, Error, ListCirclesRequest, SortDir,
};

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
            max_details: 80,
            page_size: 100,
        }
    }
}

/// Result of one scan.
#[derive(Debug, Clone, PartialEq)]
pub struct Scan {
    /// Clubs with member data, climbers first.
    pub clubs: Vec<Club>,
    /// Clubs that matched the rank window and had open spots.
    pub candidates: usize,
    /// Clubs whose details failed to load.
    pub failed: usize,
}

/// Why a scan failed.
#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    /// A list request failed.
    #[error("listing clubs failed: {0}")]
    List(#[source] Error),
}

impl ScanError {
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
pub async fn scan(client: &Client, config: &ScanConfig) -> Result<Scan, ScanError> {
    let mut candidates = list_candidates(client, config).await?;
    let total = candidates.len();

    candidates.sort_by_key(|c| Reverse(climb(c)));
    candidates.truncate(config.max_details as usize);

    let mut clubs = Vec::with_capacity(candidates.len());
    let mut failed = 0;
    for listed in candidates {
        let circle_id = listed.circle_id;
        match client.circle(&CircleRequest::by_circle_id(circle_id)).await {
            Ok(mut response) => {
                if response.circle.circle_id == 0 {
                    response.circle = listed;
                }
                clubs.push(Club::from(response));
            }
            Err(error) => {
                failed += 1;
                tracing::warn!(circle_id, %error, "skipping club that failed to load");
            }
        }
    }

    tracing::info!(
        candidates = total,
        loaded = clubs.len(),
        failed,
        "scan finished"
    );
    Ok(Scan {
        clubs,
        candidates: total,
        failed,
    })
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

        candidates.extend(rows.into_iter().filter(|c| is_candidate(c, config)));
        if done {
            break;
        }
    }
    Ok(candidates)
}

fn is_candidate(circle: &Circle, config: &ScanConfig) -> bool {
    let in_window = circle
        .monthly_rank
        .and_then(|r| u32::try_from(r).ok())
        .is_some_and(|r| (config.rank_from..=config.rank_to).contains(&r));
    let members = circle
        .member_count
        .filter(|n| *n > 0)
        .unwrap_or(CLUB_CAP as i32);
    in_window && members < CLUB_CAP as i32
}

fn climb(circle: &Circle) -> i64 {
    let last = circle.last_month_rank.map_or(1_000_000_000, i64::from);
    let now = circle.monthly_rank.map_or(0, i64::from);
    last - now
}

#[cfg(test)]
mod tests;
