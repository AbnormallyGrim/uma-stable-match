use super::*;
use std::collections::HashMap;
use std::time::Duration;

fn lookup(vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let vars: HashMap<String, String> = vars
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    move |name| vars.get(name).cloned()
}

#[test]
fn defaults() {
    let config = Config::from_lookup(lookup(&[])).unwrap();
    assert_eq!(config.bind_addr, "127.0.0.1:8080".parse().unwrap());
    assert_eq!(config.scan, ScanConfig::default());
    assert_eq!(config.scan_interval, None);
    assert_eq!(config.detail_cache, None);
    assert_eq!(config.detail_cache_mb, DEFAULT_DETAIL_CACHE_MB);
    assert_eq!(config.matching, MatchSettings::default());
    assert_eq!(config.trainer_cache, Duration::from_secs(10 * 60));
    assert_eq!(config.rate_limit_per_minute, 30);
    assert_eq!(config.trainer_lookups_per_minute, 10);
    assert!(!config.trust_forwarded_for);
    assert!(!config.features.japanese);
}

#[test]
fn reads_all_settings() {
    let config = Config::from_lookup(lookup(&[
        ("UMAMOE_API_KEY", " test-key "),
        ("UMAMOE_BASE_URL", "http://localhost:9000"),
        ("HOST", "0.0.0.0"),
        ("PORT", "9090"),
        ("TRAINER_CACHE_MINUTES", "15"),
    ]))
    .unwrap();
    assert_eq!(config.bind_addr, "0.0.0.0:9090".parse().unwrap());
    assert_eq!(config.trainer_cache, Duration::from_secs(15 * 60));
}

#[test]
fn reads_umamoe_settings() {
    let config = UmaMoeConfig::from_lookup(lookup(&[
        ("UMAMOE_API_KEY", " test-key "),
        ("UMAMOE_BASE_URL", "http://localhost:9000"),
    ]))
    .unwrap();
    assert_eq!(config.api_key, ApiKey::new("test-key"));
    assert_eq!(config.base_url.as_deref(), Some("http://localhost:9000"));
    assert_eq!(config.client_config().base_url, "http://localhost:9000");

    let config = UmaMoeConfig::from_lookup(lookup(&[
        ("UMAMOE_API_KEY", "test-key"),
        ("UMAMOE_BASE_URL", ""),
    ]))
    .unwrap();
    assert_eq!(config.base_url, None);
}

#[test]
fn missing_or_blank_key_fails() {
    assert_eq!(
        UmaMoeConfig::from_lookup(lookup(&[])).unwrap_err(),
        ConfigError::MissingApiKey
    );
    assert_eq!(
        UmaMoeConfig::from_lookup(lookup(&[("UMAMOE_API_KEY", "  ")])).unwrap_err(),
        ConfigError::MissingApiKey
    );
}

#[test]
fn server_settings_need_no_api_key() {
    assert!(Config::from_lookup(lookup(&[])).is_ok());
}

#[test]
fn ipv6_host() {
    let config = Config::from_lookup(lookup(&[
        ("UMAMOE_API_KEY", "test-key"),
        ("HOST", "::"),
        ("PORT", "8080"),
    ]))
    .unwrap();
    assert_eq!(config.bind_addr, "[::]:8080".parse().unwrap());
}

#[test]
fn invalid_host_fails() {
    for value in ["localhost", "not-an-address", "127.0.0.1:8080"] {
        let error = Config::from_lookup(lookup(&[("UMAMOE_API_KEY", "test-key"), ("HOST", value)]))
            .unwrap_err();
        assert_eq!(error, ConfigError::InvalidHost(value.into()));
    }
}

#[test]
fn invalid_port_fails() {
    for value in ["0", "65536", "-1", "http"] {
        let error = Config::from_lookup(lookup(&[("UMAMOE_API_KEY", "test-key"), ("PORT", value)]))
            .unwrap_err();
        assert_eq!(error, ConfigError::InvalidPort(value.into()));
    }
}

#[test]
fn invalid_trainer_cache_fails() {
    let error = Config::from_lookup(lookup(&[
        ("UMAMOE_API_KEY", "test-key"),
        ("TRAINER_CACHE_MINUTES", "0"),
    ]))
    .unwrap_err();
    assert!(matches!(
        error,
        ConfigError::InvalidNumber {
            name: "TRAINER_CACHE_MINUTES",
            ..
        }
    ));
}

#[test]
fn blank_optional_settings_use_defaults() {
    let config = Config::from_lookup(lookup(&[
        ("UMAMOE_API_KEY", "test-key"),
        ("UMAMOE_BASE_URL", ""),
        ("HOST", " "),
        ("PORT", ""),
    ]))
    .unwrap();
    assert_eq!(
        config.bind_addr,
        SocketAddr::new(DEFAULT_HOST, DEFAULT_PORT)
    );
}

#[test]
fn debug_and_errors_hide_the_key() {
    let config = Config::from_lookup(lookup(&[("UMAMOE_API_KEY", "test-key")])).unwrap();
    assert!(!format!("{config:?}").contains("test-key"));

    let error = Config::from_lookup(lookup(&[("UMAMOE_API_KEY", "test-key"), ("PORT", "bad")]))
        .unwrap_err();
    assert!(!error.to_string().contains("test-key"));
}

#[test]
fn reads_scan_settings() {
    let config = Config::from_lookup(lookup(&[
        ("UMAMOE_API_KEY", "test-key"),
        ("SCAN_RANK_FROM", "10"),
        ("SCAN_RANK_TO", "500"),
        ("SCAN_MAX_DETAILS", "25"),
    ]))
    .unwrap();
    assert_eq!(config.scan.rank_from, 10);
    assert_eq!(config.scan.rank_to, 500);
    assert_eq!(config.scan.max_details, 25);
    assert_eq!(config.scan.page_size, ScanConfig::default().page_size);
}

#[test]
fn invalid_scan_numbers_fail() {
    for value in ["0", "-5", "abc", "1.5"] {
        let error = Config::from_lookup(lookup(&[
            ("UMAMOE_API_KEY", "test-key"),
            ("SCAN_MAX_DETAILS", value),
        ]))
        .unwrap_err();
        assert_eq!(
            error,
            ConfigError::InvalidNumber {
                name: "SCAN_MAX_DETAILS",
                value: value.into()
            }
        );
    }
}

#[test]
fn inverted_rank_window_fails() {
    let error = Config::from_lookup(lookup(&[
        ("UMAMOE_API_KEY", "test-key"),
        ("SCAN_RANK_FROM", "600"),
        ("SCAN_RANK_TO", "500"),
    ]))
    .unwrap_err();
    assert_eq!(error, ConfigError::InvalidRankWindow { from: 600, to: 500 });
}

#[test]
fn reads_scan_interval() {
    let config = Config::from_lookup(lookup(&[
        ("UMAMOE_API_KEY", "test-key"),
        ("SCAN_INTERVAL_MINUTES", "15"),
    ]))
    .unwrap();
    assert_eq!(config.scan_interval, Some(Duration::from_secs(15 * 60)));

    let error = Config::from_lookup(lookup(&[
        ("UMAMOE_API_KEY", "test-key"),
        ("SCAN_INTERVAL_MINUTES", "0"),
    ]))
    .unwrap_err();
    assert!(matches!(
        error,
        ConfigError::InvalidNumber {
            name: "SCAN_INTERVAL_MINUTES",
            ..
        }
    ));
}

#[test]
fn reads_rate_limit_settings() {
    let config = Config::from_lookup(lookup(&[
        ("UMAMOE_API_KEY", "test-key"),
        ("RATE_LIMIT_PER_MINUTE", "60"),
        ("TRAINER_LOOKUPS_PER_MINUTE", "5"),
        ("TRUST_FORWARDED_FOR", "true"),
    ]))
    .unwrap();
    assert_eq!(config.rate_limit_per_minute, 60);
    assert_eq!(config.trainer_lookups_per_minute, 5);
    assert!(config.trust_forwarded_for);
}

#[test]
fn invalid_rate_limit_settings_fail() {
    let error = Config::from_lookup(lookup(&[
        ("UMAMOE_API_KEY", "test-key"),
        ("RATE_LIMIT_PER_MINUTE", "0"),
    ]))
    .unwrap_err();
    assert!(matches!(
        error,
        ConfigError::InvalidNumber {
            name: "RATE_LIMIT_PER_MINUTE",
            ..
        }
    ));

    let error = Config::from_lookup(lookup(&[
        ("UMAMOE_API_KEY", "test-key"),
        ("TRUST_FORWARDED_FOR", "yes"),
    ]))
    .unwrap_err();
    assert_eq!(
        error,
        ConfigError::InvalidBool {
            name: "TRUST_FORWARDED_FOR",
            value: "yes".into()
        }
    );
}

#[test]
fn reads_a_valid_frontend_dir() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("index.html"), "<!doctype html>").unwrap();
    let path = dir.path().to_str().unwrap().to_owned();
    let config = Config::from_lookup(lookup(&[
        ("UMAMOE_API_KEY", "test-key"),
        ("FRONTEND_DIR", &path),
    ]))
    .unwrap();
    assert_eq!(config.frontend_dir.as_deref(), Some(dir.path()));
}

#[test]
fn frontend_dir_is_optional() {
    let config = Config::from_lookup(lookup(&[("UMAMOE_API_KEY", "test-key")])).unwrap();
    assert_eq!(config.frontend_dir, None);
}

#[test]
fn rejects_a_frontend_dir_without_index_html() {
    let empty = tempfile::tempdir().unwrap();
    for value in [
        empty.path().to_str().unwrap().to_owned(),
        empty.path().join("missing").to_str().unwrap().to_owned(),
    ] {
        let error = Config::from_lookup(lookup(&[
            ("UMAMOE_API_KEY", "test-key"),
            ("FRONTEND_DIR", &value),
        ]))
        .unwrap_err();
        assert_eq!(error, ConfigError::InvalidFrontendDir(value));
    }
}

#[test]
fn detail_cache_is_unset_by_default_whatever_the_scan_interval() {
    let config = Config::from_lookup(lookup(&[
        ("UMAMOE_API_KEY", "test-key"),
        ("SCAN_INTERVAL_MINUTES", "45"),
    ]))
    .unwrap();
    assert_eq!(config.detail_cache, None);
}

#[test]
fn reads_detail_cache() {
    let config = Config::from_lookup(lookup(&[
        ("UMAMOE_API_KEY", "test-key"),
        ("SCAN_INTERVAL_MINUTES", "45"),
        ("DETAIL_CACHE_MINUTES", "90"),
    ]))
    .unwrap();
    assert_eq!(config.detail_cache, Some(Duration::from_secs(90 * 60)));
}

#[test]
fn reads_detail_cache_size() {
    let config = Config::from_lookup(lookup(&[
        ("UMAMOE_API_KEY", "test-key"),
        ("DETAIL_CACHE_MB", "384"),
    ]))
    .unwrap();
    assert_eq!(config.detail_cache_mb, 384);
}

#[test]
fn invalid_detail_cache_size_fails() {
    for value in ["0", "-5", "abc"] {
        let error = Config::from_lookup(lookup(&[
            ("UMAMOE_API_KEY", "test-key"),
            ("DETAIL_CACHE_MB", value),
        ]))
        .unwrap_err();
        assert_eq!(
            error,
            ConfigError::InvalidNumber {
                name: "DETAIL_CACHE_MB",
                value: value.into()
            }
        );
    }
}

#[test]
fn invalid_detail_cache_fails() {
    for value in ["0", "-5", "abc"] {
        let error = Config::from_lookup(lookup(&[
            ("UMAMOE_API_KEY", "test-key"),
            ("DETAIL_CACHE_MINUTES", value),
        ]))
        .unwrap_err();
        assert_eq!(
            error,
            ConfigError::InvalidNumber {
                name: "DETAIL_CACHE_MINUTES",
                value: value.into()
            }
        );
    }
}

#[test]
fn reads_match_settings() {
    let config = Config::from_lookup(lookup(&[
        ("UMAMOE_API_KEY", "test-key"),
        ("SHORTLIST_SIZE", "40"),
        ("REQUEST_FETCH_BUDGET", "3"),
    ]))
    .unwrap();
    assert_eq!(config.matching.shortlist_size, 40);
    assert_eq!(config.matching.fetch_budget, 3);
}

#[test]
fn invalid_match_settings_fail() {
    for name in ["SHORTLIST_SIZE", "REQUEST_FETCH_BUDGET"] {
        for value in ["0", "-1", "lots"] {
            let error =
                Config::from_lookup(lookup(&[("UMAMOE_API_KEY", "test-key"), (name, value)]))
                    .unwrap_err();
            assert_eq!(
                error,
                ConfigError::InvalidNumber {
                    name,
                    value: value.into()
                }
            );
        }
    }
}

#[test]
fn redis_url_is_optional() {
    let config = Config::from_lookup(lookup(&[("UMAMOE_API_KEY", "test-key")])).unwrap();
    assert_eq!(config.redis_url, None);
}

#[test]
fn reads_redis_url() {
    let config = Config::from_lookup(lookup(&[
        ("UMAMOE_API_KEY", "test-key"),
        ("REDIS_URL", "redis://127.0.0.1:6379"),
        ("REDIS_KEY_SECRET", "made-up-secret-made-up-secret-0001"),
    ]))
    .unwrap();
    assert_eq!(config.redis_url.unwrap().expose(), "redis://127.0.0.1:6379");
    assert!(config.redis_key_secret.is_some());
}

#[test]
fn invalid_redis_url_fails_without_echoing_it() {
    let error = Config::from_lookup(lookup(&[
        ("UMAMOE_API_KEY", "test-key"),
        (
            "REDIS_URL",
            "http://user:made-up-password@cache.example.com",
        ),
    ]))
    .unwrap_err();
    assert_eq!(error, ConfigError::InvalidRedisUrl);
    assert!(!error.to_string().contains("made-up-password"));
}

#[test]
fn config_debug_hides_the_redis_url() {
    let config = Config::from_lookup(lookup(&[
        ("UMAMOE_API_KEY", "test-key"),
        (
            "REDIS_URL",
            "redis://user:made-up-password@cache.example.com:6379",
        ),
        ("REDIS_KEY_SECRET", "made-up-secret-made-up-secret-0001"),
    ]))
    .unwrap();
    let debug = format!("{config:?}");
    assert!(!debug.contains("made-up-password"));
    assert!(!debug.contains("made-up-secret"));
}

#[test]
fn redis_needs_a_key_secret() {
    let error = Config::from_lookup(lookup(&[
        ("UMAMOE_API_KEY", "test-key"),
        ("REDIS_URL", "redis://127.0.0.1:6379"),
    ]))
    .unwrap_err();
    assert_eq!(error, ConfigError::MissingRedisKeySecret);
}

#[test]
fn short_key_secrets_fail_without_echoing_them() {
    let error = Config::from_lookup(lookup(&[
        ("UMAMOE_API_KEY", "test-key"),
        ("REDIS_URL", "redis://127.0.0.1:6379"),
        ("REDIS_KEY_SECRET", "too-short-made-up"),
    ]))
    .unwrap_err();
    assert_eq!(error, ConfigError::InvalidRedisKeySecret);
    assert!(!error.to_string().contains("too-short"));
}

#[test]
fn key_secret_is_ignored_without_redis() {
    let config = Config::from_lookup(lookup(&[
        ("UMAMOE_API_KEY", "test-key"),
        ("REDIS_KEY_SECRET", "short"),
    ]))
    .unwrap();
    assert!(config.redis_key_secret.is_none());
}

#[test]
fn metrics_are_off_unless_an_address_is_set() {
    let config = Config::from_lookup(lookup(&[("UMAMOE_API_KEY", "test-key")])).unwrap();
    assert_eq!(config.metrics_addr, None);

    let config = Config::from_lookup(lookup(&[
        ("UMAMOE_API_KEY", "test-key"),
        ("METRICS_ADDR", "127.0.0.1:9464"),
    ]))
    .unwrap();
    assert_eq!(config.metrics_addr, Some("127.0.0.1:9464".parse().unwrap()));
}

#[test]
fn metrics_address_needs_an_ip_and_port() {
    for value in ["localhost:9464", "127.0.0.1", "127.0.0.1:0", "9464"] {
        let error = Config::from_lookup(lookup(&[
            ("UMAMOE_API_KEY", "test-key"),
            ("METRICS_ADDR", value),
        ]))
        .unwrap_err();
        assert_eq!(error, ConfigError::InvalidMetricsAddr(value.into()));
    }
}

#[test]
fn reads_the_lower_range() {
    let config = Config::from_lookup(lookup(&[
        ("UMAMOE_API_KEY", "test-key"),
        ("SCAN_RANK_TO", "500"),
        ("SCAN_LOWER_RANK_TO", "2000"),
    ]))
    .unwrap();
    assert_eq!(config.scan.lower_to, Some(2000));
    assert_eq!(config.scan.last_rank(), 2000);

    let config = Config::from_lookup(lookup(&[("UMAMOE_API_KEY", "test-key")])).unwrap();
    assert_eq!(config.scan.lower_to, None);
}

#[test]
fn a_lower_range_above_the_window_fails() {
    let error = Config::from_lookup(lookup(&[
        ("UMAMOE_API_KEY", "test-key"),
        ("SCAN_RANK_TO", "500"),
        ("SCAN_LOWER_RANK_TO", "500"),
    ]))
    .unwrap_err();
    assert_eq!(
        error,
        ConfigError::InvalidLowerRange {
            lower: 500,
            to: 500
        }
    );
}

#[test]
fn the_human_check_is_off_unless_both_keys_are_set() {
    let base = ("UMAMOE_API_KEY", "test-key");
    let config = Config::from_lookup(lookup(&[base])).unwrap();
    assert!(config.human.is_none());
    assert!(!config.human_check().is_enabled());

    let config = Config::from_lookup(lookup(&[
        base,
        ("TURNSTILE_SITE_KEY", "made-up-site-key"),
        ("TURNSTILE_SECRET_KEY", "made-up-secret"),
        ("TOKEN_SIGNING_KEY", &"k".repeat(32)),
    ]))
    .unwrap();
    assert!(config.human_check().is_enabled());
    assert!(!format!("{config:?}").contains("made-up-secret"));
}

#[test]
fn rejects_a_partial_or_short_human_check() {
    let base = ("UMAMOE_API_KEY", "test-key");
    for vars in [
        vec![base, ("TURNSTILE_SECRET_KEY", "made-up-secret")],
        vec![base, ("TURNSTILE_SITE_KEY", "made-up-site-key")],
        vec![base, ("TOKEN_SIGNING_KEY", &"k".repeat(32))],
    ] {
        assert_eq!(
            Config::from_lookup(lookup(&vars)).unwrap_err(),
            ConfigError::IncompleteHumanCheck
        );
    }
    assert_eq!(
        Config::from_lookup(lookup(&[
            base,
            ("TURNSTILE_SITE_KEY", "made-up-site-key"),
            ("TURNSTILE_SECRET_KEY", "made-up-secret"),
            ("TOKEN_SIGNING_KEY", "too-short"),
        ]))
        .unwrap_err(),
        ConfigError::InvalidTokenSigningKey
    );
}

#[test]
fn reads_the_japanese_flag() {
    let config = Config::from_lookup(lookup(&[
        ("UMAMOE_API_KEY", "test-key"),
        ("ENABLE_JAPANESE", "true"),
    ]))
    .unwrap();
    assert!(config.features.japanese);
}

#[test]
fn invalid_japanese_flag_fails() {
    let error = Config::from_lookup(lookup(&[
        ("UMAMOE_API_KEY", "test-key"),
        ("ENABLE_JAPANESE", "1"),
    ]))
    .unwrap_err();
    assert_eq!(
        error,
        ConfigError::InvalidBool {
            name: "ENABLE_JAPANESE",
            value: "1".into()
        }
    );
}
