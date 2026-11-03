//! Club store tests. All data is made up.
//!
//! These need Redis and run only when `REDIS_TEST_URL` is set, for example
//! `REDIS_TEST_URL=redis://127.0.0.1:6379 cargo test`; CI sets it.

use std::sync::atomic::{AtomicU64, Ordering};

use redis::aio::ConnectionManager;
use uma_stable_match_discord::{ClubStore, Departure};

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
    assert!(!record.auto_join && !record.open && record.category.is_none());
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
    assert!(record.leader_roles.is_empty());
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
