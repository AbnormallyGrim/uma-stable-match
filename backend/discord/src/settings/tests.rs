use super::*;

fn categories() -> Vec<Named> {
    vec![Named {
        id: "10".into(),
        name: "Applications".into(),
    }]
}

fn roles(count: u64) -> Vec<NamedRole> {
    (1..=count)
        .map(|id| NamedRole {
            id,
            name: format!("role {id}"),
        })
        .collect()
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
    let update = checked(
        &patch(r#"{"category":"10","leader_roles":["2","1","2"],"open":false}"#),
        &categories(),
        &roles(3),
    )
    .unwrap();
    assert_eq!(
        update,
        SettingsUpdate {
            category: Some(Some(10)),
            leader_roles: Some(vec![2, 1]),
            auto_join: None,
            open: Some(false),
        }
    );
}

#[test]
fn refuses_other_categories_roles_and_empty_changes() {
    let check = |json| checked(&patch(json), &categories(), &roles(3));
    assert!(check(r#"{"category":"11"}"#).is_err());
    assert!(check(r#"{"category":"x"}"#).is_err());
    assert!(check(r#"{"leader_roles":["4"]}"#).is_err());
    assert!(check(r#"{"leader_roles":["x"]}"#).is_err());
    assert!(check("{}").is_err());
}

#[test]
fn limits_the_number_of_leader_roles() {
    let ids = |n: u64| {
        let ids: Vec<String> = (1..=n).map(|id| format!("\"{id}\"")).collect();
        format!("{{\"leader_roles\":[{}]}}", ids.join(","))
    };
    let all = roles(MAX_LEADER_ROLES as u64 + 1);
    assert!(checked(&patch(&ids(MAX_LEADER_ROLES as u64)), &categories(), &all).is_ok());
    assert_eq!(
        checked(
            &patch(&ids(MAX_LEADER_ROLES as u64 + 1)),
            &categories(),
            &all
        ),
        Err("too many leader roles")
    );
}
