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
    let config = Config::from_lookup(lookup(&[("UMAMOE_API_KEY", "test-key")])).unwrap();
    assert_eq!(config.api_key, ApiKey::new("test-key"));
    assert_eq!(config.bind_addr, DEFAULT_BIND_ADDR.parse().unwrap());
    assert_eq!(config.base_url, None);
    assert_eq!(config.scan, ScanConfig::default());
    assert_eq!(config.scan_interval, Duration::from_secs(180 * 60));
}

#[test]
fn reads_all_settings() {
    let config = Config::from_lookup(lookup(&[
        ("UMAMOE_API_KEY", " test-key "),
        ("UMAMOE_BASE_URL", "http://localhost:9000"),
        ("BIND_ADDR", "0.0.0.0:8080"),
    ]))
    .unwrap();
    assert_eq!(config.api_key, ApiKey::new("test-key"));
    assert_eq!(config.base_url.as_deref(), Some("http://localhost:9000"));
    assert_eq!(config.bind_addr, "0.0.0.0:8080".parse().unwrap());
}

#[test]
fn missing_or_blank_key_fails() {
    assert_eq!(
        Config::from_lookup(lookup(&[])).unwrap_err(),
        ConfigError::MissingApiKey
    );
    assert_eq!(
        Config::from_lookup(lookup(&[("UMAMOE_API_KEY", "  ")])).unwrap_err(),
        ConfigError::MissingApiKey
    );
}

#[test]
fn invalid_bind_addr_fails() {
    let error = Config::from_lookup(lookup(&[
        ("UMAMOE_API_KEY", "test-key"),
        ("BIND_ADDR", "not-an-address"),
    ]))
    .unwrap_err();
    assert_eq!(error, ConfigError::InvalidBindAddr("not-an-address".into()));
}

#[test]
fn blank_optional_settings_use_defaults() {
    let config = Config::from_lookup(lookup(&[
        ("UMAMOE_API_KEY", "test-key"),
        ("UMAMOE_BASE_URL", ""),
        ("BIND_ADDR", " "),
    ]))
    .unwrap();
    assert_eq!(config.base_url, None);
    assert_eq!(config.bind_addr, DEFAULT_BIND_ADDR.parse().unwrap());
}

#[test]
fn debug_and_errors_hide_the_key() {
    let config = Config::from_lookup(lookup(&[("UMAMOE_API_KEY", "test-key")])).unwrap();
    assert!(!format!("{config:?}").contains("test-key"));

    let error = Config::from_lookup(lookup(&[
        ("UMAMOE_API_KEY", "test-key"),
        ("BIND_ADDR", "bad"),
    ]))
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
    assert_eq!(config.scan_interval, Duration::from_secs(15 * 60));

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
