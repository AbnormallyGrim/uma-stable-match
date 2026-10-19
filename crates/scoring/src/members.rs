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

pub(crate) fn last_day(daily_fans: &[i64]) -> Option<usize> {
    daily_fans.iter().rposition(|v| *v != 0)
}

pub(crate) fn days_with_data(daily_fans: &[i64]) -> usize {
    daily_fans.iter().filter(|v| **v != 0).count()
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
