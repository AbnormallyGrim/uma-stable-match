use super::*;

#[test]
fn reads_the_answer_buttons() {
    assert_eq!(
        parse_button("apply:accept:abc_123"),
        Some((Status::Accepted, "abc_123"))
    );
    assert_eq!(
        parse_button("apply:decline:a-b"),
        Some((Status::Declined, "a-b"))
    );
}

#[test]
fn ignores_other_buttons() {
    assert_eq!(parse_button("apply:maybe:abc"), None);
    assert_eq!(parse_button("apply:accept:"), None);
    assert_eq!(parse_button("other:accept:abc"), None);
    assert_eq!(parse_button(""), None);
}
