use super::*;

const CLUB: u64 = 900_001;

fn sealer() -> Sealer {
    Sealer::new(&KeySecret::parse("made-up-key-secret-of-32-chars!!").unwrap())
}

fn parse(fields: &HashMap<String, String>) -> Option<ClubRecord> {
    parse_record(&sealer(), CLUB, fields)
}

fn fields(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

#[test]
fn reads_a_full_record() {
    let record = parse(&fields(&[
        ("server", "10"),
        ("category", "20"),
        ("leader_roles", "30,31"),
        ("auto_join", "1"),
        ("open", "1"),
        ("problem", "missing permission"),
        ("leader", &sealer().seal("40", &leader_context(CLUB))),
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
    assert_eq!(parse(&fields(&[])), None);
    assert_eq!(parse(&fields(&[("server", "abc")])), None);
    assert_eq!(
        parse(&fields(&[("server", "10"), ("problem", "")])).unwrap(),
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

#[test]
fn reads_a_leader_saved_before_leaders_were_encrypted() {
    let record = parse(&fields(&[("server", "10"), ("leader", "40")])).unwrap();
    assert_eq!(record.leader, Some(40));
}

#[test]
fn a_leader_encrypted_for_another_club_or_altered_is_ignored() {
    let other = sealer().seal("40", &leader_context(CLUB + 1));
    let record = parse(&fields(&[("server", "10"), ("leader", &other)])).unwrap();
    assert_eq!(record.leader, None);
    let mut altered = sealer().seal("40", &leader_context(CLUB));
    altered.push('A');
    let record = parse(&fields(&[("server", "10"), ("leader", &altered)])).unwrap();
    assert_eq!(record.leader, None);
}
