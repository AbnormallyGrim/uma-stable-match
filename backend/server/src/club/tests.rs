use super::*;
use uma_stable_match_scoring::MemberSnapshot;

fn member(viewer_id: u64, daily_fans: Vec<i64>) -> MemberSnapshot {
    let mut member = MemberSnapshot::new(viewer_id, daily_fans);
    member.year = Some(2026);
    member.month = Some(3);
    member
}

#[test]
fn lists_current_members_fastest_first_and_new_ones_last() {
    let mut club = Club::new(1, "Made-up Club");
    club.members = vec![
        member(1, vec![100, 200, 300]),
        member(2, vec![0, 0, 500]),
        member(3, vec![100, 400, 700]),
        member(4, vec![-100, 150, 200]),
        member(5, vec![100, 200, 0]),
    ];
    let members = members(&club);
    let paces: Vec<Option<f64>> = members.iter().map(|m| m.pace_per_day).collect();
    assert_eq!(paces, [Some(300.0), Some(100.0), Some(50.0), None]);
    assert_eq!(members[2].daily_fans, [100, 150, 200]);
}
