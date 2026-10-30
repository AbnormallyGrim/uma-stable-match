//! Member rules: daily pace and current members.

use crate::model::MemberSnapshot;

/// Days of data needed to calculate a pace.
pub const MIN_DAYS_FOR_PACE: usize = 3;

/// Average daily fan gain from a cumulative series, or `None` with too few days of data.
pub fn daily_pace(daily_fans: &[i64]) -> Option<f64> {
    // Skip days without data; previous-club values (negative) still count.
    let values: Vec<i64> = daily_fans
        .iter()
        .filter(|v| **v != 0)
        .map(|v| v.abs())
        .collect();
    if values.len() < MIN_DAYS_FOR_PACE {
        return None;
    }
    let first = values[0] as f64;
    let last = values[values.len() - 1] as f64;
    Some((last - first) / (values.len() - 1) as f64)
}

/// This month's pace blended with a prior, weighted by days of data; see the README.
///
/// Without a prior it's [`daily_pace`]. `None` if `k` isn't finite and at least 0, or there's
/// nothing to go on.
pub fn blended_pace(daily_fans: &[i64], prior: Option<f64>, k: f64) -> Option<f64> {
    if !(k.is_finite() && k >= 0.0) {
        return None;
    }
    let Some(prior) = prior.filter(|p| p.is_finite()) else {
        return daily_pace(daily_fans);
    };
    let values: Vec<i64> = daily_fans
        .iter()
        .filter(|v| **v != 0)
        .map(|v| v.abs())
        .collect();
    let gains = values.len().saturating_sub(1) as f64;
    if gains == 0.0 {
        return Some(prior);
    }
    let this_month = (values[values.len() - 1] - values[0]) as f64 / gains;
    let weight = gains / (gains + k);
    Some(weight * this_month + (1.0 - weight) * prior)
}

pub(crate) fn last_day(daily_fans: &[i64]) -> Option<usize> {
    daily_fans.iter().rposition(|v| *v != 0)
}

/// Members still in the club: those with a positive value on the club's latest data day.
pub fn current_members(members: &[MemberSnapshot]) -> Vec<&MemberSnapshot> {
    // Only the latest month counts.
    let latest_period = members
        .iter()
        .filter_map(|m| Some((m.year?, m.month?)))
        .max();
    let in_period: Vec<&MemberSnapshot> = members
        .iter()
        .filter(|m| match latest_period {
            Some(period) => m.year.zip(m.month) == Some(period),
            None => true,
        })
        .collect();

    let latest_day = in_period
        .iter()
        .filter_map(|m| last_day(&m.daily_fans))
        .max()
        .unwrap_or(0);

    in_period
        .into_iter()
        .filter(|m| m.daily_fans.get(latest_day).is_some_and(|v| *v > 0))
        .collect()
}

#[cfg(test)]
mod tests;
