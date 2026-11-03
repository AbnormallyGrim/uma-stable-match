use super::*;
use ed25519_dalek::{Signer, SigningKey};

fn keys() -> (SigningKey, PublicKey) {
    let signing = SigningKey::from_bytes(&[7; 32]);
    let public = PublicKey::parse(&hex(signing.verifying_key().as_bytes())).unwrap();
    (signing, public)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn at(seconds: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(seconds)
}

#[test]
fn accepts_a_valid_signature() {
    let (signing, public) = keys();
    let body = br#"{"type":1}"#;
    let signature = hex(&signing
        .sign(&[b"1700000000".as_slice(), body].concat())
        .to_bytes());
    assert!(verify(
        &public,
        &signature,
        "1700000000",
        body,
        at(1_700_000_000)
    ));
}

#[test]
fn rejects_a_changed_body_or_timestamp() {
    let (signing, public) = keys();
    let body = br#"{"type":1}"#;
    let signature = hex(&signing
        .sign(&[b"1700000000".as_slice(), body].concat())
        .to_bytes());
    assert!(!verify(
        &public,
        &signature,
        "1700000000",
        br#"{"type":3}"#,
        at(1_700_000_000)
    ));
    assert!(!verify(
        &public,
        &signature,
        "1700000001",
        body,
        at(1_700_000_000)
    ));
}

#[test]
fn rejects_another_key() {
    let (_, public) = keys();
    let other = SigningKey::from_bytes(&[8; 32]);
    let body = br#"{"type":1}"#;
    let signature = hex(&other
        .sign(&[b"1700000000".as_slice(), body].concat())
        .to_bytes());
    assert!(!verify(
        &public,
        &signature,
        "1700000000",
        body,
        at(1_700_000_000)
    ));
}

#[test]
fn rejects_old_or_future_timestamps() {
    let (signing, public) = keys();
    let body = br#"{"type":1}"#;
    let signature = hex(&signing
        .sign(&[b"1700000000".as_slice(), body].concat())
        .to_bytes());
    let skew = MAX_TIMESTAMP_SKEW.as_secs();
    assert!(verify(
        &public,
        &signature,
        "1700000000",
        body,
        at(1_700_000_000 + skew)
    ));
    assert!(!verify(
        &public,
        &signature,
        "1700000000",
        body,
        at(1_700_000_001 + skew)
    ));
    assert!(!verify(
        &public,
        &signature,
        "1700000000",
        body,
        at(1_699_999_999 - skew)
    ));
}

#[test]
fn rejects_malformed_headers() {
    let (_, public) = keys();
    let now = at(1_700_000_000);
    assert!(!verify(&public, "not-hex", "1700000000", b"{}", now));
    assert!(!verify(&public, "abcd", "1700000000", b"{}", now));
    assert!(!verify(&public, &"00".repeat(64), "soon", b"{}", now));
}
