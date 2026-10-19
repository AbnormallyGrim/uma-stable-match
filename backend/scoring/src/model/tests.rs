use super::*;

#[test]
fn tier_labels_cover_every_code() {
    assert_eq!(tier_label(0), None);
    assert_eq!(tier_label(1), Some("D"));
    assert_eq!(tier_label(6), Some("B+"));
    assert_eq!(tier_label(11), Some("SS"));
    assert_eq!(tier_label(12), None);
}

#[test]
fn join_style_codes() {
    assert_eq!(JoinStyle::from_code(1), Some(JoinStyle::Open));
    assert_eq!(JoinStyle::from_code(2), Some(JoinStyle::Approval));
    assert_eq!(JoinStyle::from_code(3), Some(JoinStyle::Closed));
    assert_eq!(JoinStyle::from_code(0), None);
}

#[test]
fn open_spots_never_negative() {
    let mut club = Club {
        member_count: Some(27),
        ..Default::default()
    };
    assert_eq!(club.open_spots(), Some(3));
    club.member_count = Some(31);
    assert_eq!(club.open_spots(), Some(0));
    club.member_count = None;
    assert_eq!(club.open_spots(), None);
}
