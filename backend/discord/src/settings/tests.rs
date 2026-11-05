use super::*;

fn categories() -> Vec<Named> {
    vec![Named {
        id: "10".into(),
        name: "Applications".into(),
    }]
}

fn patch(json: &str) -> Patch {
    serde_json::from_str(json).unwrap()
}

#[test]
fn tells_a_null_category_from_a_missing_one() {
    assert_eq!(patch("{}").category, None);
    assert_eq!(patch(r#"{"category":null}"#).category, Some(None));
    assert_eq!(
        patch(r#"{"category":"10"}"#).category,
        Some(Some("10".into()))
    );
}

#[test]
fn unknown_fields_are_rejected() {
    assert!(serde_json::from_str::<Patch>(r#"{"problem":"x"}"#).is_err());
}

#[test]
fn builds_an_update_from_valid_settings() {
    let update = checked(&patch(r#"{"category":"10","open":false}"#), &categories()).unwrap();
    assert_eq!(
        update,
        SettingsUpdate {
            category: Some(Some(10)),
            auto_join: None,
            open: Some(false),
        }
    );
}

#[test]
fn refuses_other_categories_and_empty_changes() {
    let check = |json| checked(&patch(json), &categories());
    assert!(check(r#"{"category":"11"}"#).is_err());
    assert!(check(r#"{"category":"x"}"#).is_err());
    assert!(check("{}").is_err());
}

#[test]
fn leader_roles_are_no_longer_a_setting() {
    assert!(serde_json::from_str::<Patch>(r#"{"leader_roles":["1"]}"#).is_err());
}
