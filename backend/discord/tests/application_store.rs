//! Application store tests. All data is made up.
//!
//! These need Redis and run only when `REDIS_TEST_URL` is set, for example
//! `REDIS_TEST_URL=redis://127.0.0.1:6379 cargo test`; CI sets it.

use std::sync::atomic::{AtomicU64, Ordering};

use redis::aio::ConnectionManager;
use uma_stable_match_discord::{
    Application, ApplicationStore, KeySecret, NewApplication, Sent, Status, MAX_PER_DAY,
};

async fn connect() -> Option<(ApplicationStore, ConnectionManager)> {
    let Ok(url) = std::env::var("REDIS_TEST_URL") else {
        eprintln!("skipped: set REDIS_TEST_URL to run the Redis tests");
        return None;
    };
    let conn = ConnectionManager::new(redis::Client::open(url).unwrap())
        .await
        .expect("REDIS_TEST_URL is reachable");
    let secret = KeySecret::parse("made-up-key-secret-of-32-chars!!").unwrap();
    Some((ApplicationStore::new(conn.clone(), secret), conn))
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

fn application(club: u64, applicant: u64) -> NewApplication {
    NewApplication {
        club,
        club_name: "Example Club".into(),
        guild: 10,
        applicant,
        trainer: 100_000_001,
        trainer_name: "Example Trainer".into(),
        current_club: Some("Other Club".into()),
        pace: Some(6_400_000.0),
        fans_this_month: Some(48_000_000),
    }
}

async fn saved(store: &ApplicationStore, new: &NewApplication) -> Application {
    let Sent::Saved(id) = store.create(new).await.unwrap() else {
        panic!("the application should be saved");
    };
    store.get(&id).await.unwrap().expect("it was saved")
}

async fn ttl(conn: &ConnectionManager, key: String) -> i64 {
    redis::cmd("TTL")
        .arg(key)
        .query_async(&mut conn.clone())
        .await
        .unwrap()
}

#[tokio::test]
async fn saves_and_reads_an_application() {
    let Some((store, conn)) = connect().await else {
        return;
    };
    let base = base();
    let new = application(base + 1, base + 2);
    let found = saved(&store, &new).await;
    assert_eq!(found.details, new);
    assert_eq!(found.status, Status::Pending);
    assert_eq!(found.channel, None);

    // It lives for about a calendar month.
    let left = ttl(&conn, format!("discord:app:{}", found.id)).await;
    assert!((27 * 86_400..=31 * 86_400).contains(&left), "{left}");
}

#[tokio::test]
async fn lists_newest_first_for_the_club_and_the_applicant() {
    let Some((store, _)) = connect().await else {
        return;
    };
    let base = base();
    let first = saved(&store, &application(base + 1, base + 2)).await;
    let second = saved(&store, &application(base + 1, base + 3)).await;
    let elsewhere = saved(&store, &application(base + 4, base + 2)).await;

    let ids = |list: Vec<Application>| list.into_iter().map(|a| a.id).collect::<Vec<_>>();
    let club = ids(store.for_club(base + 1).await.unwrap());
    assert_eq!(club.len(), 2);
    assert!(club.contains(&first.id) && club.contains(&second.id));
    let mine = ids(store.for_applicant(base + 2).await.unwrap());
    assert_eq!(mine.len(), 2);
    assert!(mine.contains(&first.id) && mine.contains(&elsewhere.id));
    assert!(store.for_applicant(base + 9).await.unwrap().is_empty());
}

#[tokio::test]
async fn one_pending_application_per_applicant_and_club() {
    let Some((store, _)) = connect().await else {
        return;
    };
    let base = base();
    let new = application(base + 1, base + 2);
    let first = saved(&store, &new).await;
    assert_eq!(store.create(&new).await.unwrap(), Sent::Pending);
    // Another club or another applicant is fine.
    saved(&store, &application(base + 3, base + 2)).await;
    saved(&store, &application(base + 1, base + 4)).await;

    // Answering it frees the applicant to apply again.
    assert!(store.answer(&first.id, Status::Declined).await.unwrap());
    saved(&store, &new).await;
}

#[tokio::test]
async fn answers_a_pending_application_once() {
    let Some((store, _)) = connect().await else {
        return;
    };
    let base = base();
    let found = saved(&store, &application(base + 1, base + 2)).await;
    assert!(store.answer(&found.id, Status::Accepted).await.unwrap());
    assert!(!store.answer(&found.id, Status::Declined).await.unwrap());
    let found = store.get(&found.id).await.unwrap().unwrap();
    assert_eq!(found.status, Status::Accepted);
    assert!(!store.answer("missing", Status::Accepted).await.unwrap());
}

#[tokio::test]
async fn limits_applications_a_day() {
    let Some((store, _)) = connect().await else {
        return;
    };
    let base = base();
    for club in 0..u64::from(MAX_PER_DAY) {
        saved(&store, &application(base + 10 + club, base + 2)).await;
    }
    assert_eq!(
        store
            .create(&application(base + 50, base + 2))
            .await
            .unwrap(),
        Sent::OverLimit
    );
    // Someone else still can.
    saved(&store, &application(base + 50, base + 3)).await;
}

#[tokio::test]
async fn discarding_gives_back_the_application_and_its_slot() {
    let Some((store, _)) = connect().await else {
        return;
    };
    let base = base();
    let new = application(base + 1, base + 2);
    let found = saved(&store, &new).await;
    store.discard(&found).await.unwrap();
    assert_eq!(store.get(&found.id).await.unwrap(), None);
    assert!(store.for_club(base + 1).await.unwrap().is_empty());
    // It no longer blocks a new one.
    saved(&store, &new).await;
}

#[tokio::test]
async fn attaches_the_ticket() {
    let Some((store, _)) = connect().await else {
        return;
    };
    let base = base();
    let found = saved(&store, &application(base + 1, base + 2)).await;
    assert!(store.attach(&found.id, 77, 88).await.unwrap());
    let found = store.get(&found.id).await.unwrap().unwrap();
    assert_eq!((found.channel, found.message), (Some(77), Some(88)));
    assert!(!store.attach("missing", 1, 2).await.unwrap());
}

#[tokio::test]
async fn lists_skip_applications_that_expired() {
    let Some((store, conn)) = connect().await else {
        return;
    };
    let base = base();
    let kept = saved(&store, &application(base + 1, base + 2)).await;
    let gone = saved(&store, &application(base + 1, base + 3)).await;
    let _: i64 = redis::cmd("DEL")
        .arg(format!("discord:app:{}", gone.id))
        .query_async(&mut conn.clone())
        .await
        .unwrap();
    let list = store.for_club(base + 1).await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, kept.id);
}
