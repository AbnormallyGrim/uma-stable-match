//! Ad store tests. All data is made up.
//!
//! These need Redis and run only when `REDIS_TEST_URL` is set, for example
//! `REDIS_TEST_URL=redis://127.0.0.1:6379 cargo test`; CI sets it.

use std::sync::atomic::{AtomicU64, Ordering};

use redis::aio::ConnectionManager;
use uma_stable_match_discord::{
    Ad, AdStore, DiscordWish, Invite, Invited, KeySecret, Language, NewAd, MAX_INVITES, STORED_FOR,
    VISIBLE_FOR,
};

async fn connect() -> Option<(AdStore, ConnectionManager)> {
    let Ok(url) = std::env::var("REDIS_TEST_URL") else {
        eprintln!("skipped: set REDIS_TEST_URL to run the Redis tests");
        return None;
    };
    let conn = ConnectionManager::new(redis::Client::open(url).unwrap())
        .await
        .expect("REDIS_TEST_URL is reachable");
    let secret = KeySecret::parse("made-up-key-secret-of-32-chars!!").unwrap();
    Some((AdStore::new(conn.clone(), secret), conn))
}

/// A base for IDs no other test, or run of the tests, uses.
fn base() -> u64 {
    static CALLS: AtomicU64 = AtomicU64::new(0);
    let run = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
        % 1_000_000_000;
    (run * 1_000 + CALLS.fetch_add(1, Ordering::Relaxed)) * 100
}

fn new_ad(user: u64) -> NewAd {
    NewAd {
        user,
        trainer: 100_000_001,
        trainer_name: "Example Trainer".into(),
        current_club: Some("Other Club".into()),
        pace: Some(6_400_000.0),
        fans_this_month: Some(48_000_000),
        rank_min: 3,
        rank_max: 8,
        discord: DiscordWish::Preferred,
        language: Language::English,
    }
}

async fn ttl(conn: &ConnectionManager, id: &str) -> i64 {
    redis::cmd("TTL")
        .arg(format!("discord:ad:{id}"))
        .query_async(&mut conn.clone())
        .await
        .unwrap()
}

/// Moves the ad off the board without deleting it, as the passing of time does.
async fn hide(conn: &ConnectionManager, ad: &Ad) {
    let past = 1_000;
    let mut conn = conn.clone();
    let _: () = redis::cmd("HSET")
        .arg(format!("discord:ad:{}", ad.id))
        .arg("visible_until")
        .arg(past)
        .query_async(&mut conn)
        .await
        .unwrap();
    let _: () = redis::cmd("ZADD")
        .arg("discord:ads")
        .arg(past)
        .arg(&ad.id)
        .query_async(&mut conn)
        .await
        .unwrap();
}

#[tokio::test]
async fn an_ad_is_kept_and_on_the_board_for_a_week() {
    let Some((store, conn)) = connect().await else {
        return;
    };
    let user = base();
    let ad = store.save(&new_ad(user)).await.unwrap();
    assert_eq!(ad.details, new_ad(user));
    assert_eq!(ad.visible_until - ad.created_at, VISIBLE_FOR.as_secs());
    let left = ttl(&conn, &ad.id).await;
    assert!(left > STORED_FOR.as_secs() as i64 - 60 && left <= STORED_FOR.as_secs() as i64);
    assert!(store.board().await.unwrap().iter().any(|a| a.id == ad.id));
    assert_eq!(store.get(user).await.unwrap(), Some(ad));
    store.delete(user).await.unwrap();
}

#[tokio::test]
async fn an_ad_off_the_board_stays_to_be_renewed() {
    let Some((store, conn)) = connect().await else {
        return;
    };
    let user = base();
    let ad = store.save(&new_ad(user)).await.unwrap();
    hide(&conn, &ad).await;
    assert!(!store.board().await.unwrap().iter().any(|a| a.id == ad.id));
    let kept = store.get(user).await.unwrap().expect("it is still kept");
    assert_eq!(kept.visible_until, 1_000);
    // Saving again puts it back on the board, as it was first posted.
    let renewed = store.save(&new_ad(user)).await.unwrap();
    assert!(renewed.visible_until > kept.visible_until);
    assert_eq!(renewed.created_at, ad.created_at);
    assert!(store.board().await.unwrap().iter().any(|a| a.id == ad.id));
    store.delete(user).await.unwrap();
}

#[tokio::test]
async fn saving_again_replaces_the_ad_and_keeps_its_invites() {
    let Some((store, _)) = connect().await else {
        return;
    };
    let user = base();
    let ad = store.save(&new_ad(user)).await.unwrap();
    assert_eq!(store.invite(&ad.id, 7).await.unwrap(), Invited::Sent);
    let changed = NewAd {
        current_club: None,
        pace: None,
        rank_max: 11,
        language: Language::Korean,
        ..new_ad(user)
    };
    let saved = store.save(&changed).await.unwrap();
    assert_eq!(saved.details, changed);
    assert_eq!(saved.invites.len(), 1);
    assert_eq!(saved.invites[0].club, 7);
    store.delete(user).await.unwrap();
}

#[tokio::test]
async fn a_club_invites_a_trainer_once() {
    let Some((store, _)) = connect().await else {
        return;
    };
    let user = base();
    let ad = store.save(&new_ad(user)).await.unwrap();
    assert_eq!(store.invite(&ad.id, 7).await.unwrap(), Invited::Sent);
    assert_eq!(store.invite(&ad.id, 7).await.unwrap(), Invited::Already);
    let kept = store.get(user).await.unwrap().unwrap();
    assert_eq!(kept.invites.iter().map(|i| i.club).collect::<Vec<_>>(), [7]);
    store.delete(user).await.unwrap();
}

#[tokio::test]
async fn an_ad_takes_so_many_invites_until_one_is_dismissed() {
    let Some((store, _)) = connect().await else {
        return;
    };
    let user = base();
    let ad = store.save(&new_ad(user)).await.unwrap();
    for club in 1..=MAX_INVITES as u64 {
        assert_eq!(store.invite(&ad.id, club).await.unwrap(), Invited::Sent);
    }
    assert_eq!(store.invite(&ad.id, 99).await.unwrap(), Invited::Full);
    assert!(store.dismiss(user, 1).await.unwrap());
    assert!(!store.dismiss(user, 1).await.unwrap());
    assert_eq!(store.invite(&ad.id, 99).await.unwrap(), Invited::Sent);
    let kept = store.get(user).await.unwrap().unwrap();
    assert_eq!(kept.invites.len(), MAX_INVITES);
    assert!(kept.invites.iter().all(|Invite { club, .. }| *club != 1));
    store.delete(user).await.unwrap();
}

#[tokio::test]
async fn nobody_is_invited_from_an_ad_that_is_gone_or_off_the_board() {
    let Some((store, conn)) = connect().await else {
        return;
    };
    let user = base();
    assert_eq!(
        store
            .invite("00000000000000000000000000000000", 7)
            .await
            .unwrap(),
        Invited::Gone
    );
    let ad = store.save(&new_ad(user)).await.unwrap();
    hide(&conn, &ad).await;
    assert_eq!(store.invite(&ad.id, 7).await.unwrap(), Invited::Gone);
    store.delete(user).await.unwrap();
}

#[tokio::test]
async fn deleting_an_ad_takes_its_invites_and_its_place_on_the_board() {
    let Some((store, conn)) = connect().await else {
        return;
    };
    let user = base();
    let ad = store.save(&new_ad(user)).await.unwrap();
    store.invite(&ad.id, 7).await.unwrap();
    assert!(store.delete(user).await.unwrap());
    assert!(!store.delete(user).await.unwrap());
    assert_eq!(store.get(user).await.unwrap(), None);
    assert!(!store.board().await.unwrap().iter().any(|a| a.id == ad.id));
    assert_eq!(ttl(&conn, &ad.id).await, -2);
}

#[tokio::test]
async fn an_ad_is_hashed_by_its_trainers_discord_id() {
    let Some((store, _)) = connect().await else {
        return;
    };
    let (first, second) = (base(), base());
    let a = store.save(&new_ad(first)).await.unwrap();
    let b = store.save(&new_ad(second)).await.unwrap();
    assert_ne!(a.id, b.id);
    assert!(!a.id.contains(&first.to_string()));
    store.delete(first).await.unwrap();
    store.delete(second).await.unwrap();
}
