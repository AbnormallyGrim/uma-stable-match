use super::*;

#[test]
fn api_key_debug_is_redacted() {
    let key = ApiKey::new("not-a-real-key");
    assert_eq!(format!("{key:?}"), "ApiKey(<redacted>)");
}

#[test]
fn config_debug_does_not_show_key() {
    let config = ClientConfig::new(ApiKey::new("not-a-real-key"));
    assert!(!format!("{config:?}").contains("not-a-real-key"));
}

#[test]
fn defaults_point_at_uma_moe() {
    let config = ClientConfig::new(ApiKey::new("k"));
    assert_eq!(config.base_url, DEFAULT_BASE_URL);
    assert!(config.user_agent.starts_with("uma-stable-match/"));
}
