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
            leader: None,
            ticket_name: None,
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

#[test]
fn builds_a_ticket_name_update() {
    let name = |json: &str| {
        checked(&patch(json), &categories())
            .unwrap()
            .ticket_name
            .unwrap()
    };
    assert_eq!(
        name(r#"{"ticket_name":{"mode":"default"}}"#),
        TicketName::Default
    );
    assert_eq!(
        name(r#"{"ticket_name":{"mode":"default","value":null}}"#),
        TicketName::Default
    );
    assert_eq!(
        name(r#"{"ticket_name":{"mode":"prefix","value":"Apply Here"}}"#),
        TicketName::Prefix("apply-here".into())
    );
    assert_eq!(
        name(r#"{"ticket_name":{"mode":"override","value":"join-us"}}"#),
        TicketName::Override("join-us".into())
    );
}

#[test]
fn refuses_ticket_names_that_do_not_fit_their_mode() {
    for json in [
        r#"{"ticket_name":{"mode":"default","value":"x"}}"#,
        r#"{"ticket_name":{"mode":"prefix"}}"#,
        r#"{"ticket_name":{"mode":"override","value":null}}"#,
        r#"{"ticket_name":{"mode":"prefix","value":""}}"#,
        r#"{"ticket_name":{"mode":"prefix","value":"'; DROP TABLE clubs;--"}}"#,
        r#"{"ticket_name":{"mode":"override","value":"<script>"}}"#,
        r#"{"ticket_name":{"mode":"prefix","value":"apply\r\nSET x y"}}"#,
    ] {
        assert!(checked(&patch(json), &categories()).is_err(), "{json}");
    }
}

#[test]
fn refuses_malformed_ticket_name_settings() {
    for json in [
        r#"{"ticket_name":"apply"}"#,
        r#"{"ticket_name":{}}"#,
        r#"{"ticket_name":{"mode":"nope"}}"#,
        r#"{"ticket_name":{"mode":"prefix","value":5}}"#,
        r#"{"ticket_name":{"mode":"prefix","value":["a"]}}"#,
        r#"{"ticket_name":{"mode":"prefix","value":"a","extra":1}}"#,
        r#"{"ticket_name":{"mode":"Prefix","value":"a"}}"#,
        r#"{"ticket_prefix":"apply"}"#,
    ] {
        assert!(serde_json::from_str::<Patch>(json).is_err(), "{json}");
    }
}

#[test]
fn a_null_ticket_name_changes_nothing() {
    assert!(checked(&patch(r#"{"ticket_name":null}"#), &categories()).is_err());
}

#[test]
fn a_ticket_name_alone_is_a_change() {
    let update = checked(
        &patch(r#"{"ticket_name":{"mode":"default"}}"#),
        &categories(),
    )
    .unwrap();
    assert_eq!(update.ticket_name, Some(TicketName::Default));
}

#[test]
fn shows_the_ticket_name_as_a_mode_and_a_value() {
    let show = |name: &TicketName| serde_json::to_value(TicketNameSetting::of(name)).unwrap();
    assert_eq!(
        show(&TicketName::Default),
        serde_json::json!({ "mode": "default", "value": null })
    );
    assert_eq!(
        show(&TicketName::Prefix("apply".into())),
        serde_json::json!({ "mode": "prefix", "value": "apply" })
    );
    assert_eq!(
        show(&TicketName::Override("join".into())),
        serde_json::json!({ "mode": "override", "value": "join" })
    );
}
