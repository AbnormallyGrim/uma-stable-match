use super::*;

#[test]
fn keys_include_the_month() {
    assert_eq!(club_key(900001, None), "usm:v1:club:900001:current");
    assert_eq!(
        club_key(900001, Some((2026, 3))),
        "usm:v1:club:900001:2026-03"
    );
    let none = SharedCache::none();
    assert_eq!(none.pace_key(4242, None), "usm:v1:pace:4242:current");
    assert_eq!(
        none.pace_key(4242, Some((2026, 12))),
        "usm:v1:pace:4242:2026-12"
    );
}

#[test]
fn redis_urls_need_a_redis_scheme() {
    assert!(RedisUrl::parse("redis://127.0.0.1:6379").is_some());
    assert!(RedisUrl::parse("rediss://cache.example.com:6380/0").is_some());
    assert!(RedisUrl::parse("redis://").is_none());
    assert!(RedisUrl::parse("http://127.0.0.1:6379").is_none());
    assert!(RedisUrl::parse("127.0.0.1:6379").is_none());
}

#[test]
fn redis_url_debug_hides_credentials() {
    let url = RedisUrl::parse("redis://user:made-up-password@cache.example.com:6379").unwrap();
    assert_eq!(format!("{url:?}"), "RedisUrl(<redacted>)");
    assert_eq!(
        url.expose(),
        "redis://user:made-up-password@cache.example.com:6379"
    );
}

#[tokio::test]
async fn the_default_cache_stores_nothing() {
    let cache = SharedCache::none();
    assert!(!cache.is_enabled());
    cache
        .set("usm:v1:test", &42u32, Duration::from_secs(60))
        .await;
    assert_eq!(cache.get::<u32>("usm:v1:test").await, None);
}

const SECRET: &str = "made-up-secret-made-up-secret-0001";

#[test]
fn key_secrets_need_32_characters() {
    assert!(KeySecret::parse(SECRET).is_some());
    assert!(KeySecret::parse(&"x".repeat(32)).is_some());
    assert!(KeySecret::parse(&"x".repeat(31)).is_none());
    assert_eq!(
        format!("{:?}", KeySecret::parse(SECRET).unwrap()),
        "KeySecret(<redacted>)"
    );
}

#[test]
fn trainer_digests_are_keyed_and_hide_the_id() {
    let secret = KeySecret::parse(SECRET).unwrap();
    let other = KeySecret::parse("another-made-up-secret-0000000002").unwrap();
    let digest = trainer_digest(&secret, 100000000001);
    assert_eq!(digest.len(), 32);
    assert!(digest.chars().all(|c| c.is_ascii_hexdigit()));
    assert!(!digest.contains("100000000001"));
    assert_eq!(digest, trainer_digest(&secret, 100000000001));
    assert_ne!(digest, trainer_digest(&secret, 100000000002));
    assert_ne!(digest, trainer_digest(&other, 100000000001));
}

#[test]
fn scan_keys_depend_on_the_scan_window() {
    let keys = ScanKeys::new(&ScanConfig::default());
    assert_eq!(keys.lock, "usm:v1:scan:80-900-200:lock");
    assert_eq!(keys.fresh, "usm:v1:scan:80-900-200:fresh");
    assert_eq!(keys.latest, "usm:v1:scan:80-900-200:latest");
    assert_eq!(keys.progress, "usm:v1:scan:80-900-200:progress");
    assert_eq!(keys.progress_id, "usm:v1:scan:80-900-200:progress-id");
    let other = ScanKeys::new(&ScanConfig {
        rank_to: 500,
        ..ScanConfig::default()
    });
    assert_eq!(other.lock, "usm:v1:scan:80-500-200:lock");
}

#[tokio::test]
async fn without_redis_every_instance_scans_alone() {
    let cache = SharedCache::none();
    assert!(matches!(
        cache.scan_turn(&ScanConfig::default()).await,
        ScanTurn::Alone
    ));
    assert_eq!(
        cache.published_scan::<u32>(&ScanConfig::default()).await,
        None
    );
}

fn utc(text: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(text).unwrap().into()
}

#[test]
fn schedule_scores_hold_the_due_time_and_rank() {
    let due = utc("2026-03-19T15:30:00Z");
    assert_eq!(decode(score(due, 42) as f64), (due, 42));
    assert_eq!(decode(score(due, u32::MAX) as f64), (due, u32::MAX));
}

#[test]
fn schedule_scores_order_by_due_time_then_rank() {
    let due = utc("2026-03-19T15:30:00Z");
    let later = utc("2026-03-19T15:30:01Z");
    assert!(score(due, 1) < score(due, 2));
    assert!(score(due, u32::MAX) < score(later, 1));
}

#[test]
fn schedule_scores_round_due_times_up_to_the_second() {
    let due = utc("2026-03-19T15:30:00.250Z");
    assert_eq!(decode(score(due, 7) as f64).0, utc("2026-03-19T15:30:01Z"));
}

#[test]
fn schedule_scores_stay_exact_as_floats() {
    let due = utc("2099-12-31T23:59:59Z");
    let value = score(due, 2_000_000);
    assert!(value < 1 << 53);
    assert_eq!(value as f64 as u64, value);
}

#[test]
fn reads_redis_info() {
    let info = "# Memory\r\nused_memory:56900000\r\nused_memory_human:54.26M\r\nmaxmemory:314572800\r\n\r\n# Stats\r\nexpired_keys:120\r\nevicted_keys:3\r\n\r\n# Keyspace\r\ndb0:keys=4988,expires=4986,avg_ttl=150000000,subexpiry=0\r\ndb1:keys=2,expires=2,avg_ttl=50000000,subexpiry=0\r\n";
    let stats = RedisStats::parse(info);
    assert_eq!(stats.keys, 4990);
    assert_eq!(stats.expiring, 4988);
    assert_eq!(stats.used_memory, 56_900_000);
    assert_eq!(stats.max_memory, 314_572_800);
    assert_eq!(stats.expired, 120);
    assert_eq!(stats.evicted, 3);
    let expected = (150_000_000u64 * 4986 + 50_000_000 * 2) / 4988;
    assert_eq!(stats.average_ttl, Duration::from_millis(expected));
}

#[test]
fn reads_an_empty_redis_info() {
    assert_eq!(RedisStats::parse("# Keyspace\r\n"), RedisStats::default());
}
