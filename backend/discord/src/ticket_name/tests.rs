use super::*;

fn prefix(text: &str) -> Result<String, NameError> {
    match TicketName::with_prefix(text)? {
        TicketName::Prefix(prefix) => Ok(prefix),
        other => panic!("not a prefix: {other:?}"),
    }
}

#[test]
fn names_a_ticket_after_the_club_and_the_trainer_by_default() {
    let name = TicketName::Default.channel_name("Example Club", "Example Trainer", 482);
    assert_eq!(name, "example-club-example-trainer-0482");
}

#[test]
fn a_prefix_stands_in_for_the_club() {
    let name = TicketName::with_prefix("Apply Here").unwrap().channel_name(
        "Example Club",
        "Example Trainer",
        7,
    );
    assert_eq!(name, "apply-here-example-trainer-0007");
}

#[test]
fn an_override_is_the_name_and_the_number() {
    let name = TicketName::with_override("join-us").unwrap().channel_name(
        "Example Club",
        "Example Trainer",
        9_999,
    );
    assert_eq!(name, "join-us-9999");
}

#[test]
fn cleans_up_names_from_uma_moe() {
    let name = TicketName::Default.channel_name("  Ex--Club!! ", "  Ex--Tra!! ", 1);
    assert_eq!(name, "ex-club-ex-tra-0001");
}

#[test]
fn leaves_out_a_club_with_no_letters_and_names_a_trainer_with_none() {
    let name = TicketName::Default.channel_name("サークル", "トレーナー", 12);
    assert_eq!(name, "applicant-0012");
    let name = TicketName::Default.channel_name("Example Club", "トレーナー", 12);
    assert_eq!(name, "example-club-applicant-0012");
}

#[test]
fn the_number_wraps_to_four_digits() {
    let name = TicketName::Default.channel_name("a", "b", 10_001);
    assert_eq!(name, "a-b-0001");
}

#[test]
fn the_club_gives_way_to_the_trainer_and_the_number() {
    let trainer = "t".repeat(200);
    let name = TicketName::Default.channel_name(&"c".repeat(200), &trainer, 5);
    assert!(name.len() <= MAX_NAME, "{}", name.len());
    assert!(name.ends_with(&format!("-{}-0005", "t".repeat(MAX_TRAINER))));

    let name = TicketName::with_prefix(&"p".repeat(MAX_PREFIX))
        .unwrap()
        .channel_name("", &trainer, 5);
    assert!(name.len() <= MAX_NAME);
    assert!(name.starts_with(&"p".repeat(MAX_PREFIX)));
}

#[test]
fn no_name_ends_a_part_with_a_dash_or_runs_past_the_limit() {
    // Cut where the club's slug has a dash.
    let club = format!("{} x", "c".repeat(43));
    let name = TicketName::Default.channel_name(&club, &"t".repeat(40), 5);
    assert!(!name.contains("--"), "{name}");
    assert!(name.len() <= MAX_NAME);
    let longest = TicketName::with_override(&"o".repeat(MAX_OVERRIDE)).unwrap();
    assert!(longest.channel_name("", "", 5).len() <= MAX_NAME);
}

#[test]
fn takes_letters_digits_and_single_dashes() {
    assert_eq!(prefix("apply"), Ok("apply".into()));
    assert_eq!(prefix("Apply 2 Join_Us"), Ok("apply-2-join-us".into()));
    assert_eq!(prefix("  apply  "), Ok("apply".into()));
    assert_eq!(prefix(&"a".repeat(MAX_PREFIX)), Ok("a".repeat(MAX_PREFIX)));
}

#[test]
fn refuses_what_it_would_have_to_change() {
    for text in [
        "'; DROP TABLE clubs;--",
        "\" OR \"1\"=\"1",
        "<script>alert(1)</script>",
        "{{7*7}}",
        "${jndi:ldap://x}",
        "%s%n",
        "../../etc/passwd",
        "a/b",
        "a\\b",
        "nul\0l",
        "two\nlines",
        "carriage\r\nreturn",
        "tab\tbed",
        "]]; redis.call('FLUSHALL') --",
        "\u{202e}gnp",
        "zero\u{200b}width",
        "\u{ff41}\u{ff42}",
        "аpple",
        "apply 🎉",
        "トレーナー",
        "-leading",
        "trailing-",
        "double--dash",
        "---",
        "a - b",
        "@everyone",
        "#general",
        "a.b",
    ] {
        assert_eq!(
            TicketName::with_prefix(text),
            Err(NameError::BadCharacters),
            "{text:?}"
        );
        assert_eq!(
            TicketName::with_override(text),
            Err(NameError::BadCharacters),
            "{text:?}"
        );
    }
}

#[test]
fn refuses_nothing_and_too_much() {
    for text in ["", "   ", "\t"] {
        assert_eq!(
            TicketName::with_prefix(text),
            Err(NameError::Empty),
            "{text:?}"
        );
    }
    assert_eq!(
        TicketName::with_prefix(&"a".repeat(MAX_PREFIX + 1)),
        Err(NameError::TooLong)
    );
    assert!(TicketName::with_override(&"a".repeat(MAX_PREFIX + 1)).is_ok());
    assert_eq!(
        TicketName::with_override(&"a".repeat(MAX_OVERRIDE + 1)),
        Err(NameError::TooLong)
    );
    assert_eq!(
        TicketName::with_override(&"a".repeat(10_000)),
        Err(NameError::TooLong)
    );
    // Too long wins over bad characters, so a huge input is never scanned.
    assert_eq!(
        TicketName::with_override(&"'".repeat(10_000)),
        Err(NameError::TooLong)
    );
}

#[test]
fn stores_one_field_and_empties_the_other() {
    let empty = String::new;
    assert_eq!(
        TicketName::Default.fields(),
        [("ticket_prefix", empty()), ("ticket_name", empty())]
    );
    assert_eq!(
        TicketName::Prefix("apply".into()).fields(),
        [("ticket_prefix", "apply".into()), ("ticket_name", empty())]
    );
    assert_eq!(
        TicketName::Override("join".into()).fields(),
        [("ticket_prefix", empty()), ("ticket_name", "join".into())]
    );
}

#[test]
fn reads_what_was_stored_and_distrusts_the_rest() {
    let read = TicketName::from_stored;
    assert_eq!(read(None, None), TicketName::Default);
    assert_eq!(read(Some(""), Some("")), TicketName::Default);
    assert_eq!(
        read(Some("apply"), None),
        TicketName::Prefix("apply".into())
    );
    assert_eq!(
        read(None, Some("join")),
        TicketName::Override("join".into())
    );
    // Both: the override wins.
    assert_eq!(
        read(Some("apply"), Some("join")),
        TicketName::Override("join".into())
    );
    // Written by hand into Redis, not by us.
    assert_eq!(read(Some("'; DROP"), None), TicketName::Default);
    assert_eq!(read(None, Some("a\r\nb")), TicketName::Default);
    assert_eq!(read(Some(&"a".repeat(500)), None), TicketName::Default);
}

#[test]
fn numbers_have_four_digits() {
    for _ in 0..1_000 {
        assert!(random_number() < 10_000);
    }
}
