use super::*;
use crate::config::{ApplicationId, BotToken, ClientSecret, PublicKey, PublicUrl, SigningKey};

fn config(fill: char) -> DiscordConfig {
    DiscordConfig::new(
        ApplicationId::parse("1000").unwrap(),
        // The Ed25519 base point: a valid public key.
        PublicKey::parse("5866666666666666666666666666666666666666666666666666666666666666")
            .unwrap(),
        BotToken::parse("made-up-bot-token").unwrap(),
        ClientSecret::parse("made-up-client-secret").unwrap(),
        SigningKey::parse(&fill.to_string().repeat(32)).unwrap(),
        PublicUrl::parse("https://example.test").unwrap(),
    )
}

fn at(seconds: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(seconds)
}

#[test]
fn state_passes_with_its_nonce() {
    let config = config('a');
    let (state, nonce) = begin(&config, Flow::Leader, "/club/1", at(1_000));
    let checked = check_state(&config, &state, [nonce.as_str()].into_iter(), at(1_001)).unwrap();
    assert_eq!(checked.f, Flow::Leader);
    assert_eq!(checked.r, "/club/1");
}

#[test]
fn state_expires_after_ten_minutes() {
    let config = config('a');
    let (state, nonce) = begin(&config, Flow::Leader, "/", at(1_000));
    let nonces = || [nonce.as_str()].into_iter();
    assert!(check_state(&config, &state, nonces(), at(1_599)).is_some());
    assert!(check_state(&config, &state, nonces(), at(1_600)).is_none());
}

#[test]
fn state_needs_the_matching_nonce() {
    let config = config('a');
    let (state, _) = begin(&config, Flow::Leader, "/", at(1_000));
    let (_, other_nonce) = begin(&config, Flow::Leader, "/", at(1_000));
    assert!(check_state(&config, &state, std::iter::empty(), at(1_001)).is_none());
    assert!(check_state(
        &config,
        &state,
        [other_nonce.as_str()].into_iter(),
        at(1_001)
    )
    .is_none());
}

#[test]
fn state_is_rejected_when_tampered_or_signed_with_another_key() {
    let config = config('a');
    let (state, nonce) = begin(&config, Flow::Leader, "/", at(1_000));
    let nonces = || [nonce.as_str()].into_iter();
    let (payload, tag) = state.split_once('.').unwrap();
    assert!(check_state(&config, &format!("{payload}x.{tag}"), nonces(), at(1_001)).is_none());
    assert!(check_state(&config, &format!("{payload}.{tag}x"), nonces(), at(1_001)).is_none());
    assert!(check_state(&config, "garbage", nonces(), at(1_001)).is_none());
    assert!(check_state(&self::config('b'), &state, nonces(), at(1_001)).is_none());
}

#[test]
fn a_session_token_is_not_a_state() {
    let config = config('a');
    let session = seal(&config.signing_key, SESSION_PURPOSE, &"123", 9_999);
    assert!(check_state(&config, &session, std::iter::empty(), at(1)).is_none());
}

#[test]
fn flows_ask_for_their_scopes() {
    assert_eq!(Flow::Leader.scopes(), "identify");
    assert_eq!(Flow::OptIn.scopes(), "bot identify");
    assert_eq!(Flow::Apply.scopes(), "identify guilds.join");
    assert_eq!(Flow::parse("opt_in"), Some(Flow::OptIn));
    assert_eq!(Flow::parse("nope"), None);
}

#[test]
fn next_must_be_a_path_on_this_site() {
    assert_eq!(safe_next("/club/1?x=2"), "/club/1?x=2");
    for bad in [
        "",
        "club",
        "//evil.test",
        "/\\evil.test",
        "https://evil.test",
        "/a\nb",
    ] {
        assert_eq!(safe_next(bad), "/", "{bad:?}");
    }
    assert_eq!(safe_next(&format!("/{}", "a".repeat(300))), "/");
}

#[test]
fn cookies_are_http_only_and_secure_over_https() {
    let config = config('a');
    let cookie = set_cookie(&config, "n", "v", "/p", Duration::from_secs(5));
    assert_eq!(
        cookie,
        "n=v; Path=/p; Max-Age=5; HttpOnly; SameSite=Lax; Secure"
    );
    let mut local = config;
    local.public_url = PublicUrl::parse("http://localhost:8080").unwrap();
    assert!(!set_cookie(&local, "n", "v", "/", Duration::ZERO)
        .to_str()
        .unwrap()
        .contains("Secure"));
}

#[test]
fn finds_every_value_of_a_cookie() {
    let mut headers = HeaderMap::new();
    headers.append(COOKIE, HeaderValue::from_static("a=1; uma=2"));
    headers.append(COOKIE, HeaderValue::from_static("uma=3"));
    assert_eq!(
        cookie_values(&headers, "uma").collect::<Vec<_>>(),
        ["2", "3"]
    );
}
