use super::*;

fn fields(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

#[test]
fn reads_a_full_record() {
    let record = parse_record(&fields(&[
        ("server", "10"),
        ("category", "20"),
        ("leader_roles", "30,31"),
        ("auto_join", "1"),
        ("open", "1"),
        ("problem", "missing permission"),
        ("leader", "40"),
    ]))
    .unwrap();
    assert_eq!(
        record,
        ClubRecord {
            guild_id: 10,
            category: Some(20),
            auto_join: true,
            open: true,
            problem: Some("missing permission".into()),
            leader: Some(40),
        }
    );
}

#[test]
fn a_record_needs_a_server_and_defaults_the_rest() {
    assert_eq!(parse_record(&fields(&[])), None);
    assert_eq!(parse_record(&fields(&[("server", "abc")])), None);
    assert_eq!(
        parse_record(&fields(&[("server", "10"), ("problem", "")])).unwrap(),
        ClubRecord {
            guild_id: 10,
            category: None,
            auto_join: false,
            open: false,
            problem: None,
            leader: None,
        }
    );
}

#[test]
fn reads_what_the_scripts_return() {
    let reply = |a: &str, b: &str| vec![a.to_owned(), b.to_owned()];
    assert_eq!(
        departure(&reply("10", "0")),
        Some(Departure {
            guild_id: 10,
            remaining: 0
        })
    );
    assert_eq!(departure(&[]), None);
    assert_eq!(departure(&reply("x", "0")), None);
}
