use super::*;

fn ad(
    id: &str,
    pace: Option<f64>,
    ranks: (u8, u8),
    discord: DiscordWish,
    language: Language,
) -> Ad {
    Ad {
        id: id.into(),
        details: NewAd {
            user: 40,
            trainer: 100_000_001,
            trainer_name: format!("Trainer {id}"),
            current_club: None,
            pace,
            fans_this_month: None,
            leader_chara_dress_id: None,
            rank_min: ranks.0,
            rank_max: ranks.1,
            discord,
            language,
        },
        created_at: 1_000,
        visible_until: 2_000,
        invites: Vec::new(),
    }
}

const EVERYTHING: BoardFilters = BoardFilters {
    rank_min: None,
    rank_max: None,
    has_discord: None,
    language: Language::Any,
};

fn ids(shown: &[&Ad]) -> Vec<String> {
    shown.iter().map(|ad| ad.id.clone()).collect()
}

fn params(pairs: &[(&str, &str)]) -> BoardParams {
    let get = |name: &str| {
        pairs
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| (*value).to_owned())
    };
    BoardParams {
        club: get("club"),
        page: get("page"),
        rank_min: get("rank_min"),
        rank_max: get("rank_max"),
        has_discord: get("has_discord"),
        language: get("language"),
    }
}

#[test]
fn filters_left_out_are_unset_and_the_page_is_the_first() {
    assert_eq!(BoardFilters::parse(&params(&[])), Ok((EVERYTHING, 1)));
}

#[test]
fn reads_every_filter_and_the_page() {
    let parsed = BoardFilters::parse(&params(&[
        ("rank_min", "3"),
        ("rank_max", "8"),
        ("has_discord", "false"),
        ("language", "ko"),
        ("page", "2"),
    ]));
    assert_eq!(
        parsed,
        Ok((
            BoardFilters {
                rank_min: Some(3),
                rank_max: Some(8),
                has_discord: Some(false),
                language: Language::Korean,
            },
            2
        ))
    );
}

#[test]
fn ranks_given_the_wrong_way_round_are_swapped() {
    let (filters, _) =
        BoardFilters::parse(&params(&[("rank_min", "8"), ("rank_max", "3")])).unwrap();
    assert_eq!((filters.rank_min, filters.rank_max), (Some(3), Some(8)));
    // One end alone stays as it is.
    let (filters, _) = BoardFilters::parse(&params(&[("rank_min", "8")])).unwrap();
    assert_eq!((filters.rank_min, filters.rank_max), (Some(8), None));
}

#[test]
fn refuses_what_it_cannot_read() {
    for bad in [
        ("rank_min", "0"),
        ("rank_max", "12"),
        ("rank_min", "high"),
        ("has_discord", "yes"),
        ("language", "xx"),
        ("page", "0"),
        ("page", "-1"),
    ] {
        assert!(BoardFilters::parse(&params(&[bad])).is_err(), "{bad:?}");
    }
}

#[test]
fn each_rank_filter_checks_its_own_end_of_the_ad() {
    let ads = [
        ad("low", Some(4.0), (1, 3), DiscordWish::Any, Language::Any),
        ad("mid", Some(3.0), (5, 7), DiscordWish::Any, Language::Any),
        ad("wide", Some(2.0), (2, 10), DiscordWish::Any, Language::Any),
        ad("top", Some(1.0), (9, 11), DiscordWish::Any, Language::Any),
    ];
    // A lowest rank of A+ (8) keeps only ads that start at A+ or above.
    let filters = BoardFilters {
        rank_min: Some(8),
        ..EVERYTHING
    };
    assert_eq!(ids(&board_page(&ads, &filters, 1).0), ["top"]);
    // A highest rank of B+ (6) keeps only ads that end at B+ or below.
    let filters = BoardFilters {
        rank_max: Some(6),
        ..EVERYTHING
    };
    assert_eq!(ids(&board_page(&ads, &filters, 1).0), ["low"]);
    // Together they keep ads inside the range.
    let filters = BoardFilters {
        rank_min: Some(5),
        rank_max: Some(7),
        ..EVERYTHING
    };
    assert_eq!(ids(&board_page(&ads, &filters, 1).0), ["mid"]);
    assert_eq!(board_page(&ads, &EVERYTHING, 1).0.len(), 4);
}

#[test]
fn a_club_without_discord_loses_only_trainers_who_require_it() {
    let ads = [
        ad(
            "needs",
            Some(3.0),
            (1, 11),
            DiscordWish::Required,
            Language::Any,
        ),
        ad(
            "likes",
            Some(2.0),
            (1, 11),
            DiscordWish::Preferred,
            Language::Any,
        ),
        ad("any", Some(1.0), (1, 11), DiscordWish::Any, Language::Any),
    ];
    let filters = BoardFilters {
        has_discord: Some(false),
        ..EVERYTHING
    };
    assert_eq!(ids(&board_page(&ads, &filters, 1).0), ["likes", "any"]);
    let filters = BoardFilters {
        has_discord: Some(true),
        ..EVERYTHING
    };
    assert_eq!(board_page(&ads, &filters, 1).0.len(), 3);
    assert_eq!(board_page(&ads, &EVERYTHING, 1).0.len(), 3);
}

#[test]
fn keeps_ads_in_the_clubs_language_and_those_that_dont_mind() {
    let ads = [
        ad(
            "en",
            Some(3.0),
            (1, 11),
            DiscordWish::Any,
            Language::English,
        ),
        ad(
            "ja",
            Some(2.0),
            (1, 11),
            DiscordWish::Any,
            Language::Japanese,
        ),
        ad("any", Some(1.0), (1, 11), DiscordWish::Any, Language::Any),
    ];
    let filters = BoardFilters {
        language: Language::Japanese,
        ..EVERYTHING
    };
    assert_eq!(ids(&board_page(&ads, &filters, 1).0), ["ja", "any"]);
}

#[test]
fn lists_the_fastest_first_and_those_without_a_pace_last() {
    let ads = [
        ad("none", None, (1, 11), DiscordWish::Any, Language::Any),
        ad("slow", Some(1.0), (1, 11), DiscordWish::Any, Language::Any),
        ad("fast", Some(9.0), (1, 11), DiscordWish::Any, Language::Any),
    ];
    assert_eq!(
        ids(&board_page(&ads, &EVERYTHING, 1).0),
        ["fast", "slow", "none"]
    );
}

#[test]
fn pages_through_the_matches_and_keeps_the_page_in_range() {
    let ads: Vec<Ad> = (0..45)
        .map(|n| {
            ad(
                &n.to_string(),
                Some(f64::from(100 - n)),
                (1, 11),
                DiscordWish::Any,
                Language::Any,
            )
        })
        .collect();
    let (shown, page, pages, total) = board_page(&ads, &EVERYTHING, 1);
    assert_eq!((shown.len(), page, pages, total), (PAGE_SIZE, 1, 3, 45));
    assert_eq!(shown[0].id, "0");
    let (shown, page, ..) = board_page(&ads, &EVERYTHING, 3);
    assert_eq!((shown.len(), page), (5, 3));
    assert_eq!(shown[0].id, "40");
    // A page past the end shows the last one.
    let (_, page, ..) = board_page(&ads, &EVERYTHING, 9);
    assert_eq!(page, 3);
}

#[test]
fn nothing_matching_is_one_empty_page() {
    let (shown, page, pages, total) = board_page(&[], &EVERYTHING, 4);
    assert!(shown.is_empty());
    assert_eq!((page, pages, total), (1, 1, 0));
}
