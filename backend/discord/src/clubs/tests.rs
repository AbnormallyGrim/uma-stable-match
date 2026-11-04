use super::*;
use crate::config::{ApplicationId, BotToken, ClientSecret, PublicKey, PublicUrl, SigningKey};
use axum::http::header::COOKIE;

fn config(secret: &str) -> DiscordConfig {
    DiscordConfig::new(
        ApplicationId::parse("1000").unwrap(),
        PublicKey::parse("5866666666666666666666666666666666666666666666666666666666666666")
            .unwrap(),
        BotToken::parse("made-up-bot-token").unwrap(),
        ClientSecret::parse("made-up-client-secret").unwrap(),
        SigningKey::parse(secret).unwrap(),
        PublicUrl::parse("https://example.test").unwrap(),
    )
}

/// The request headers a browser would send back for `set`, a `Set-Cookie` value.
fn sent_back(set: &HeaderValue) -> HeaderMap {
    let pair = set.to_str().unwrap().split(';').next().unwrap().to_owned();
    let mut headers = HeaderMap::new();
    headers.insert(COOKIE, HeaderValue::from_str(&pair).unwrap());
    headers
}

const KEY: &str = "made-up-signing-key-of-32-chars!";

#[test]
fn a_hint_gives_back_the_server_for_the_same_person() {
    let config = config(KEY);
    let headers = sent_back(&hint_cookie(&config, 7, 99));
    assert_eq!(hinted_guild(&config, &headers, 7), Some(99));
}

#[test]
fn a_hint_is_no_use_to_another_person_or_without_the_key() {
    let config = config(KEY);
    let headers = sent_back(&hint_cookie(&config, 7, 99));
    assert_eq!(hinted_guild(&config, &headers, 8), None);
    let other = self::config("another-made-up-signing-key-32chars");
    assert_eq!(hinted_guild(&other, &headers, 7), None);
    assert_eq!(hinted_guild(&config, &HeaderMap::new(), 7), None);
}

#[test]
fn a_session_isnt_a_hint() {
    let config = config(KEY);
    let session = seal(&config.signing_key, "session", &"7", u64::MAX);
    let mut headers = HeaderMap::new();
    headers.insert(
        COOKIE,
        HeaderValue::from_str(&format!("{HINT_COOKIE}={session}")).unwrap(),
    );
    assert_eq!(hinted_guild(&config, &headers, 7), None);
}

#[test]
fn the_hint_cookie_is_http_only_and_lasts_a_month() {
    let value = hint_cookie(&config(KEY), 7, 99);
    let value = value.to_str().unwrap();
    assert!(
        value.contains("HttpOnly") && value.contains("Secure") && value.contains("SameSite=Lax")
    );
    assert!(value.contains(&format!("Max-Age={}", MAX_HINT_AGE.as_secs())));
    assert!(clear_hint(&config(KEY))
        .to_str()
        .unwrap()
        .contains("Max-Age=0"));
}
