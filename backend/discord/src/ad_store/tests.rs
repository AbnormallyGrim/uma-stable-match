use super::*;

const ID: &str = "0123456789abcdef0123456789abcdef";

fn sealer() -> Sealer {
    Sealer::new(&KeySecret::parse("made-up-key-secret-of-32-chars!!").unwrap())
}

fn fields(extra: &[(&str, &str)]) -> HashMap<String, String> {
    let mut fields: HashMap<String, String> = [
        ("applicant", sealer().seal("40", ID)),
        ("trainer", "100000001".into()),
        ("trainer_name", "Example Trainer".into()),
        ("rank_min", "3".into()),
        ("rank_max", "8".into()),
        ("discord", "preferred".into()),
        ("language", "en".into()),
        ("created", "1000".into()),
        ("visible_until", "2000".into()),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v))
    .collect();
    fields.extend(
        extra
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned())),
    );
    fields
}

fn parse_fields(extra: &[(&str, &str)]) -> Option<Ad> {
    parse(&sealer(), ID, &fields(extra))
}

#[test]
fn reads_an_ad() {
    let ad = parse_fields(&[
        ("current_club", "Other Club"),
        ("pace", "6400000.5"),
        ("fans", "48000000"),
        ("dress", "100101"),
    ])
    .unwrap();
    assert_eq!(
        ad,
        Ad {
            id: ID.into(),
            details: NewAd {
                user: 40,
                trainer: 100_000_001,
                trainer_name: "Example Trainer".into(),
                current_club: Some("Other Club".into()),
                pace: Some(6_400_000.5),
                fans_this_month: Some(48_000_000),
                leader_chara_dress_id: Some(100_101),
                rank_min: 3,
                rank_max: 8,
                discord: DiscordWish::Preferred,
                language: Language::English,
            },
            created_at: 1000,
            visible_until: 2000,
            invites: Vec::new(),
        }
    );
}

#[test]
fn optional_fields_may_be_missing() {
    let ad = parse_fields(&[]).unwrap();
    assert_eq!(ad.details.current_club, None);
    assert_eq!(ad.details.pace, None);
    assert_eq!(ad.details.fans_this_month, None);
    assert_eq!(ad.details.leader_chara_dress_id, None);
}

#[test]
fn lists_invites_newest_first() {
    let ad = parse_fields(&[("inv:7", "1500"), ("inv:9", "1700"), ("inv:8", "1500")]).unwrap();
    assert_eq!(
        ad.invites,
        [
            Invite {
                club: 9,
                sent_at: 1700
            },
            Invite {
                club: 7,
                sent_at: 1500
            },
            Invite {
                club: 8,
                sent_at: 1500
            },
        ]
    );
}

#[test]
fn skips_invites_it_cannot_read() {
    let ad = parse_fields(&[("inv:x", "1500"), ("inv:7", "soon"), ("inv:8", "1600")]).unwrap();
    assert_eq!(
        ad.invites,
        [Invite {
            club: 8,
            sent_at: 1600
        }]
    );
}

#[test]
fn rejects_ranks_out_of_range() {
    assert!(parse_fields(&[("rank_min", "0")]).is_none());
    assert!(parse_fields(&[("rank_max", "12")]).is_none());
    assert!(parse_fields(&[("rank_min", "9")]).is_none());
}

#[test]
fn rejects_an_ad_sealed_for_another_id() {
    let mut fields = fields(&[]);
    fields.insert("applicant".into(), sealer().seal("40", "another-id"));
    assert!(parse(&sealer(), ID, &fields).is_none());
}

#[test]
fn rejects_unknown_wishes() {
    assert!(parse_fields(&[("discord", "maybe")]).is_none());
    assert!(parse_fields(&[("language", "xx")]).is_none());
}

#[test]
fn spells_wishes_and_languages_as_the_api_does() {
    for wish in [
        DiscordWish::Required,
        DiscordWish::Preferred,
        DiscordWish::Any,
    ] {
        assert_eq!(DiscordWish::parse(wish.as_str()), Some(wish));
    }
    for language in Language::ALL {
        assert_eq!(Language::parse(language.as_str()), Some(language));
    }
}
