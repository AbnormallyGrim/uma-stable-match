use super::*;

#[test]
fn application_id_must_be_positive() {
    assert_eq!(
        ApplicationId::parse(" 42 ").map(ApplicationId::get),
        Some(42)
    );
    assert_eq!(ApplicationId::parse("0"), None);
    assert_eq!(ApplicationId::parse("abc"), None);
}

#[test]
fn public_key_must_be_32_hex_bytes() {
    // The Ed25519 base point: a valid public key.
    let valid = "5866666666666666666666666666666666666666666666666666666666666666";
    assert!(PublicKey::parse(valid).is_some());
    assert!(PublicKey::parse(&valid[2..]).is_none());
    assert!(PublicKey::parse(&format!("{valid}00")).is_none());
    assert!(PublicKey::parse(&valid.replace('6', "z")).is_none());
}

#[test]
fn secrets_are_redacted() {
    let token = BotToken::parse("made-up-token").unwrap();
    let secret = ClientSecret::parse("made-up-secret").unwrap();
    assert_eq!(format!("{token:?}"), "BotToken(<redacted>)");
    assert_eq!(format!("{secret:?}"), "ClientSecret(<redacted>)");
    assert!(BotToken::parse("  ").is_none());
    assert!(ClientSecret::parse("").is_none());
}

#[test]
fn decodes_hex() {
    assert_eq!(decode_hex("00ff7A"), Some(vec![0, 255, 122]));
    assert_eq!(decode_hex("abc"), None);
    assert_eq!(decode_hex("zz"), None);
}
