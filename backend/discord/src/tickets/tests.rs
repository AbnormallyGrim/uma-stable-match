use super::*;
use crate::application_store::{NewApplication, Status};
use crate::apply::Alltime;

fn application() -> Application {
    Application {
        id: "abc_123".into(),
        details: NewApplication {
            club: 7,
            club_name: "Example Club".into(),
            guild: 10,
            applicant: 20,
            trainer: 100_000_001,
            trainer_name: "Example Trainer".into(),
            current_club: None,
            pace: Some(6_400_000.0),
            fans_this_month: Some(48_123_456),
        },
        status: Status::Pending,
        submitted_at: 0,
        channel: None,
        message: None,
    }
}

fn applicant() -> Applicant {
    Applicant {
        name: "Example Trainer".into(),
        club_name: Some("Other Club".into()),
        pace_per_day: Some(6_400_000.0),
        fans_this_month: Some(48_123_456),
        alltime: Some(Alltime {
            total_fans: 1_500_000_000,
            active_days: 300,
            avg_day: Some(3_000_000.4),
            avg_week: Some(21_000_000.0),
            avg_month: None,
            rank: Some(1_234),
        }),
    }
}

#[test]
fn groups_digits() {
    assert_eq!(grouped(0), "0");
    assert_eq!(grouped(999), "999");
    assert_eq!(grouped(1_000), "1,000");
    assert_eq!(grouped(48_123_456), "48,123,456");
    assert_eq!(grouped(-1_234), "-1,234");
}

#[test]
fn the_message_shows_the_applicant_and_links_the_profile() {
    let body = message(&application(), &applicant(), Some(30));
    let embed = &body["embeds"][0];
    assert_eq!(embed["url"], "https://uma.moe/profile/100000001");
    let fields = embed["fields"].to_string();
    assert!(fields.contains("<@20>"));
    assert!(fields.contains("Other Club"));
    assert!(fields.contains("6,400,000/day"));
    assert!(fields.contains("48,123,456"));
    assert!(fields.contains("1,500,000,000"));
    assert!(fields.contains("Average per day"));
    assert!(fields.contains("3,000,000"));
    assert!(fields.contains("21,000,000"));
    assert!(!fields.contains("Average per month"));
    assert!(fields.contains("#1,234"));
    assert!(body["content"].as_str().unwrap().starts_with("<@30> "));
    assert_eq!(body["allowed_mentions"]["users"], json!(["20", "30"]));
}

#[test]
fn the_message_has_accept_and_decline_buttons() {
    let body = message(&application(), &applicant(), None);
    let buttons = &body["components"][0]["components"];
    assert_eq!(buttons[0]["custom_id"], "apply:accept:abc_123");
    assert_eq!(buttons[1]["custom_id"], "apply:decline:abc_123");
    assert_eq!(body["allowed_mentions"]["users"], json!(["20"]));
}

#[test]
fn leaves_out_what_isnt_known() {
    let unknown = Applicant {
        name: "Example Trainer".into(),
        club_name: None,
        pace_per_day: None,
        fans_this_month: None,
        alltime: None,
    };
    let body = message(&application(), &unknown, None);
    let fields = body["embeds"][0]["fields"].to_string();
    assert!(!fields.contains("Pace"));
    assert!(!fields.contains("Fans this month"));
    assert!(!fields.contains("Total fans"));
    assert!(fields.contains("None"));
}
