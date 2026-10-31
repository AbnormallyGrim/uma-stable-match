use super::*;

fn key(c: char) -> SigningKey {
    SigningKey::parse(&c.to_string().repeat(32)).unwrap()
}

fn at(secs: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(secs)
}

#[test]
fn secrets_are_validated_and_redacted() {
    assert!(TurnstileSecret::parse("  ").is_none());
    assert!(SigningKey::parse(&"x".repeat(31)).is_none());
    assert_eq!(
        format!("{:?}", TurnstileSecret::parse("made-up").unwrap()),
        "TurnstileSecret(<redacted>)"
    );
    assert_eq!(format!("{:?}", key('a')), "SigningKey(<redacted>)");
}

#[test]
fn a_fresh_token_is_valid_until_it_expires() {
    let token = sign(&key('a'), at(1_000));
    assert!(valid_at(&key('a'), &token, at(1_000)));
    assert!(valid_at(
        &key('a'),
        &token,
        at(1_000 + TOKEN_LIFETIME.as_secs() - 1)
    ));
    assert!(!valid_at(
        &key('a'),
        &token,
        at(1_000 + TOKEN_LIFETIME.as_secs())
    ));
}

#[test]
fn tokens_from_another_key_or_edited_are_invalid() {
    let token = sign(&key('a'), at(1_000));
    assert!(!valid_at(&key('b'), &token, at(1_000)));
    let (expires, tag) = token.split_once('.').unwrap();
    let later = format!("{}.{tag}", expires.parse::<u64>().unwrap() + 1);
    assert!(!valid_at(&key('a'), &later, at(1_000)));
}

#[test]
fn malformed_tokens_are_invalid() {
    for token in ["", ".", "abc", "1.zz", "99999999999.abc", "99999999999.é0"] {
        assert!(!valid_at(&key('a'), token, at(1_000)), "{token:?}");
    }
}

#[test]
fn an_off_check_accepts_anything() {
    let check = HumanCheck::default();
    assert!(!check.is_enabled());
    assert!(check.is_valid("nonsense"));
}

#[test]
fn every_token_cookie_is_read_and_other_cookies_are_ignored() {
    let mut headers = HeaderMap::new();
    headers.append(COOKIE, HeaderValue::from_static("other=1; usm_human=first"));
    headers.append(
        COOKIE,
        HeaderValue::from_static("usm_human=second;usm_human_x=no"),
    );
    assert_eq!(cookies(&headers).collect::<Vec<_>>(), ["first", "second"]);
    assert_eq!(cookies(&HeaderMap::new()).count(), 0);
}

#[test]
fn the_token_cookie_is_scoped_to_the_api_and_hidden_from_scripts() {
    assert_eq!(
        token_cookie("123.abc"),
        "usm_human=123.abc; Max-Age=600; Path=/api; HttpOnly; Secure; SameSite=Strict"
    );
}
