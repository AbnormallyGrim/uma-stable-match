use super::*;

#[test]
fn keys_include_the_month() {
    assert_eq!(club_key(900001, None), "usm:v1:club:900001:current");
    assert_eq!(
        club_key(900001, Some((2026, 3))),
        "usm:v1:club:900001:2026-03"
    );
    assert_eq!(pace_key(4242, None), "usm:v1:pace:4242:current");
    assert_eq!(pace_key(4242, Some((2026, 12))), "usm:v1:pace:4242:2026-12");
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
