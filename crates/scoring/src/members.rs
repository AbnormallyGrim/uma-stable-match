//! Member-level rules: daily pace and who counts as a current member.

use crate::model::MemberSnapshot;

/// Minimum number of days with data needed to calculate a pace.
pub const MIN_DAYS_FOR_PACE: usize = 3;

/// Average daily fan gain from a cumulative fan series.
///
/// Days with no data (`0`) are skipped, and fan counts recorded in a previous
/// club (negative values) are counted as normal. Returns `None` when fewer than
/// [`MIN_DAYS_FOR_PACE`] days have data.
///
/// ```
/// use uma_stable_match_scoring::daily_pace;
///
/// // 10M on day 1, 16M on day 3, 22M on day 5: 12M over 2 gaps = 6M/day
/// assert_eq!(daily_pace(&[10_000_000, 0, 16_000_000, 0, 22_000_000]), Some(6_000_000.0));
/// assert_eq!(daily_pace(&[10_000_000, 12_000_000]), None);
/// ```
pub fn daily_pace(daily_fans: &[i64]) -> Option<f64> {
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

/// Index of the last day with data, or `None` if the series is empty.
pub(crate) fn last_day(daily_fans: &[i64]) -> Option<usize> {
    daily_fans.iter().rposition(|v| *v != 0)
}

/// Number of days with data.
pub(crate) fn days_with_data(daily_fans: &[i64]) -> usize {
    daily_fans.iter().filter(|v| **v != 0).count()
}

/// Members who are still in the club.
///
/// 1. Only snapshots from the latest month present are considered.
/// 2. The club's latest data day is the last day on which any member has data.
/// 3. A member is current if their value on that day is positive. Zero means
///    they have left; a negative value means fans recorded in a previous club.
pub fn current_members(members: &[MemberSnapshot]) -> Vec<&MemberSnapshot> {
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
mod tests {
    use super::*;

    fn member(id: u64, daily_fans: &[i64]) -> MemberSnapshot {
        MemberSnapshot {
            viewer_id: id,
            year: Some(2026),
            month: Some(10),
            daily_fans: daily_fans.to_vec(),
        }
    }

    #[test]
    fn pace_skips_empty_days_and_counts_previous_club_fans() {
        assert_eq!(daily_pace(&[-100, -110, 120, 130]), Some(10.0));
        assert_eq!(daily_pace(&[0, 100, 0, 130, 160, 0]), Some(30.0));
        assert_eq!(daily_pace(&[]), None);
        assert_eq!(daily_pace(&[100, 0, 0, 200]), None);
    }

    #[test]
    fn pace_can_be_zero() {
        assert_eq!(daily_pace(&[500, 500, 500]), Some(0.0));
    }

    #[test]
    fn current_members_drop_people_who_left() {
        let members = vec![
            member(1, &[10, 20, 30, 40]),
            member(2, &[10, 20, 0, 0]), // left after day 2
            member(3, &[0, 0, 5, 6]),   // joined on day 3
        ];
        let ids: Vec<u64> = current_members(&members)
            .iter()
            .map(|m| m.viewer_id)
            .collect();
        assert_eq!(ids, vec![1, 3]);
    }

    #[test]
    fn previous_club_value_on_latest_day_is_not_current() {
        let members = vec![member(1, &[10, 20, 30]), member(2, &[-5, -6, -7])];
        let ids: Vec<u64> = current_members(&members)
            .iter()
            .map(|m| m.viewer_id)
            .collect();
        assert_eq!(ids, vec![1]);
    }

    #[test]
    fn only_latest_month_counts() {
        let mut old = member(9, &[10, 20, 30, 40]);
        old.month = Some(9);
        let members = vec![old, member(1, &[10, 20])];
        let ids: Vec<u64> = current_members(&members)
            .iter()
            .map(|m| m.viewer_id)
            .collect();
        assert_eq!(ids, vec![1]);
    }

    #[test]
    fn members_without_period_are_all_considered() {
        let mut a = member(1, &[1, 2, 3]);
        let mut b = member(2, &[1, 2, 0]);
        a.year = None;
        a.month = None;
        b.year = None;
        b.month = None;
        let ids: Vec<u64> = current_members(&[a, b])
            .iter()
            .map(|m| m.viewer_id)
            .collect();
        assert_eq!(ids, vec![1]);
    }

    #[test]
    fn empty_club_has_no_current_members() {
        assert!(current_members(&[]).is_empty());
        assert!(current_members(&[member(1, &[0, 0])]).is_empty());
    }
}
