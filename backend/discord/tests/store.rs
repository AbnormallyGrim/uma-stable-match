//! Club store tests. All data is made up.
//!
//! These need Redis and run only when `REDIS_TEST_URL` is set, for example
//! `REDIS_TEST_URL=redis://127.0.0.1:6379 cargo test`; CI sets it.

use std::sync::atomic::{AtomicU64, Ordering};

use redis::aio::ConnectionManager;
use uma_stable_match_discord::{ClubStore, Departure, SettingsUpdate};

/// A store and its connection, or `None` without `REDIS_TEST_URL`.
async fn connect() -> Option<(ClubStore, ConnectionManager)> {
    let Ok(url) = std::env::var("REDIS_TEST_URL") else {
        eprintln!("skipped: set REDIS_TEST_URL to run the Redis tests");
        return None;
    };
    let conn = ConnectionManager::new(redis::Client::open(url).unwrap())
        .await
        .expect("REDIS_TEST_URL is reachable");
    Some((ClubStore::new(conn.clone()), conn))
}

/// IDs no other test, or run of the tests, uses.
fn ids() -> (u64, u64, u64, u64) {
    static CALLS: AtomicU64 = AtomicU64::new(0);
    let run = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
        % 1_000_000_000;
    let base = (run * 1_000 + CALLS.fetch_add(1, Ordering::Relaxed)) * 10;
    (base + 1, base + 2, base + 3, base + 4)
}

async fn ttl(conn: &ConnectionManager, key: String) -> i64 {
    redis::cmd("TTL")
        .arg(key)
        .query_async(&mut conn.clone())
        .await
        .unwrap()
}

async fn set(conn: &ConnectionManager, club: u64, field: &str, value: &str) {
    redis::cmd("HSET")
        .arg(format!("discord:club:{club}"))
        .arg(field)
        .arg(value)
        .query_async::<()>(&mut conn.clone())
        .await
        .unwrap();
}

#[tokio::test]
async fn links_a_club_to_a_server_without_expiry() {
    let Some((store, conn)) = connect().await else {
        return;
    };
    let (club, _, guild, _) = ids();
    assert_eq!(store.club(club).await.unwrap(), None);
    assert_eq!(store.link(club, guild).await.unwrap(), None);

    let record = store.club(club).await.unwrap().unwrap();
    assert_eq!(record.guild_id, guild);
    assert!(record.auto_join && record.open && record.category.is_none());
    assert_eq!(store.clubs_on(guild).await.unwrap(), [club]);
    assert_eq!(ttl(&conn, format!("discord:club:{club}")).await, -1);
    assert_eq!(ttl(&conn, format!("discord:guild:{guild}")).await, -1);

    store.unlink(club).await.unwrap();
}

#[tokio::test]
async fn sibling_clubs_share_a_server_without_affecting_each_other() {
    let Some((store, conn)) = connect().await else {
        return;
    };
    let (first, second, guild, _) = ids();
    store.link(first, guild).await.unwrap();
    set(&conn, first, "open", "1").await;
    store.link(second, guild).await.unwrap();

    assert_eq!(store.clubs_on(guild).await.unwrap(), [first, second]);
    assert!(store.club(first).await.unwrap().unwrap().open);

    // Leaving takes only that club, and says another is still there.
    assert_eq!(
        store.unlink(first).await.unwrap(),
        Some(Departure {
            guild_id: guild,
            remaining: 1
        })
    );
    assert_eq!(store.clubs_on(guild).await.unwrap(), [second]);
    assert_eq!(store.club(first).await.unwrap(), None);
    assert_eq!(store.club(second).await.unwrap().unwrap().guild_id, guild);

    assert_eq!(
        store.unlink(second).await.unwrap(),
        Some(Departure {
            guild_id: guild,
            remaining: 0
        })
    );
    assert!(store.clubs_on(guild).await.unwrap().is_empty());
}

#[tokio::test]
async fn linking_again_to_the_same_server_changes_nothing() {
    let Some((store, conn)) = connect().await else {
        return;
    };
    let (club, _, guild, _) = ids();
    store.link(club, guild).await.unwrap();
    set(&conn, club, "category", "77").await;
    set(&conn, club, "problem", "made-up problem").await;

    assert_eq!(store.link(club, guild).await.unwrap(), None);
    let record = store.club(club).await.unwrap().unwrap();
    assert_eq!(record.category, Some(77));
    assert_eq!(record.problem.as_deref(), Some("made-up problem"));
    assert_eq!(store.clubs_on(guild).await.unwrap(), [club]);

    store.unlink(club).await.unwrap();
}

#[tokio::test]
async fn moving_a_club_leaves_the_old_server_and_drops_its_settings() {
    let Some((store, conn)) = connect().await else {
        return;
    };
    let (club, sibling, old, new) = ids();
    store.link(club, old).await.unwrap();
    store.link(sibling, old).await.unwrap();
    set(&conn, club, "category", "77").await;
    set(&conn, club, "leader_roles", "1,2").await;
    set(&conn, club, "problem", "made-up problem").await;
    set(&conn, club, "auto_join", "1").await;

    assert_eq!(
        store.link(club, new).await.unwrap(),
        Some(Departure {
            guild_id: old,
            remaining: 1
        })
    );
    let record = store.club(club).await.unwrap().unwrap();
    assert_eq!(record.guild_id, new);
    assert_eq!(record.category, None);
    assert_eq!(record.problem, None);
    assert!(
        record.auto_join,
        "settings that aren't about the old server stay"
    );
    assert_eq!(store.clubs_on(old).await.unwrap(), [sibling]);
    assert_eq!(store.clubs_on(new).await.unwrap(), [club]);

    store.unlink(club).await.unwrap();
    store.unlink(sibling).await.unwrap();
}

#[tokio::test]
async fn unlinking_an_unlinked_club_does_nothing() {
    let Some((store, _)) = connect().await else {
        return;
    };
    let (club, ..) = ids();
    assert_eq!(store.unlink(club).await.unwrap(), None);
}

#[tokio::test]
async fn a_new_link_is_open_with_auto_join_and_keeps_its_choices_when_moved() {
    let Some((store, _)) = connect().await else {
        return;
    };
    let (club, _, old, new) = ids();
    store.link(club, old).await.unwrap();
    let paused = SettingsUpdate {
        open: Some(false),
        auto_join: Some(false),
        ..SettingsUpdate::default()
    };
    assert!(store.update(club, old, &paused).await.unwrap());
    store.link(club, new).await.unwrap();

    let record = store.club(club).await.unwrap().unwrap();
    assert!(!record.open && !record.auto_join, "choices survive a move");
    store.unlink(club).await.unwrap();
}

#[tokio::test]
async fn updates_change_only_the_fields_given() {
    let Some((store, _)) = connect().await else {
        return;
    };
    let (club, _, guild, _) = ids();
    store.link(club, guild).await.unwrap();
    let update = SettingsUpdate {
        category: Some(Some(77)),
        ..SettingsUpdate::default()
    };
    assert!(store.update(club, guild, &update).await.unwrap());
    let record = store.club(club).await.unwrap().unwrap();
    assert_eq!(record.category, Some(77));
    assert!(record.open && record.auto_join);

    let clear = SettingsUpdate {
        category: Some(None),
        open: Some(false),
        ..SettingsUpdate::default()
    };
    assert!(store.update(club, guild, &clear).await.unwrap());
    let record = store.club(club).await.unwrap().unwrap();
    assert_eq!(record.category, None);
    assert!(!record.open && record.auto_join);
    store.unlink(club).await.unwrap();
}

#[tokio::test]
async fn updates_are_refused_for_another_server_or_an_unlinked_club() {
    let Some((store, _)) = connect().await else {
        return;
    };
    let (club, _, guild, other) = ids();
    let update = SettingsUpdate {
        open: Some(false),
        ..SettingsUpdate::default()
    };
    assert!(!store.update(club, guild, &update).await.unwrap());
    store.link(club, guild).await.unwrap();
    assert!(!store.update(club, other, &update).await.unwrap());
    assert!(store.club(club).await.unwrap().unwrap().open);
    store.unlink(club).await.unwrap();
}

#[tokio::test]
async fn reports_which_clubs_take_applications() {
    let Some((store, conn)) = connect().await else {
        return;
    };
    let (open, paused, broken, unlinked) = ids();
    let guild = unlinked + 1;
    for club in [open, paused, broken] {
        store.link(club, guild).await.unwrap();
    }
    let pause = SettingsUpdate {
        open: Some(false),
        ..SettingsUpdate::default()
    };
    store.update(paused, guild, &pause).await.unwrap();
    set(&conn, broken, "problem", "made-up problem").await;

    assert_eq!(
        store
            .takes_applications(&[open, paused, broken, unlinked])
            .await
            .unwrap(),
        [true, false, false, false]
    );
    assert!(store.takes_applications(&[]).await.unwrap().is_empty());
    for club in [open, paused, broken] {
        store.unlink(club).await.unwrap();
    }
}

#[tokio::test]
async fn a_dev_server_takes_every_clubs_applications() {
    let Some((store, _)) = connect().await else {
        return;
    };
    let (club, other, ..) = ids();
    let dev = store.clone().with_dev_guild(555);
    assert_eq!(
        dev.takes_applications(&[club, other]).await.unwrap(),
        vec![true, true]
    );
    let record = dev.club(club).await.unwrap().unwrap();
    assert_eq!(record.guild_id, 555);
    assert!(record.open && record.auto_join);
    assert_eq!(record.problem, None);
    // The store itself still knows nothing about the club.
    assert_eq!(store.club(club).await.unwrap(), None);
    assert_eq!(
        store.takes_applications(&[club]).await.unwrap(),
        vec![false]
    );
}

#[tokio::test]
async fn keeps_the_leader_who_set_the_club_up() {
    let Some((store, _)) = connect().await else {
        return;
    };
    let (club, ..) = ids();
    store.link(club, 10).await.unwrap();
    assert_eq!(store.club(club).await.unwrap().unwrap().leader, None);
    let update = SettingsUpdate {
        leader: Some(42),
        ..SettingsUpdate::default()
    };
    assert!(store.update(club, 10, &update).await.unwrap());
    assert_eq!(store.club(club).await.unwrap().unwrap().leader, Some(42));
    store.unlink(club).await.unwrap();
}
