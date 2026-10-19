use super::*;
use std::collections::HashMap;

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
