use std::time::Duration;

use super::*;

fn key(fill: char) -> SigningKey {
    SigningKey::parse(&fill.to_string().repeat(32)).unwrap()
}

fn at(seconds: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(seconds)
}

#[test]
fn opens_what_it_sealed_until_it_expires() {
    let token = seal(&key('a'), "test", &"hello", 1_000);
    assert_eq!(
        open::<String>(&key('a'), "test", &token, at(999)),
        Some("hello".into())
    );
    assert_eq!(open::<String>(&key('a'), "test", &token, at(1_000)), None);
    assert_eq!(open::<String>(&key('a'), "test", &token, at(2_000)), None);
}

#[test]
fn rejects_another_key_or_purpose() {
    let token = seal(&key('a'), "test", &"hello", 1_000);
    assert_eq!(open::<String>(&key('b'), "test", &token, at(0)), None);
    assert_eq!(open::<String>(&key('a'), "other", &token, at(0)), None);
}

#[test]
fn rejects_a_changed_payload_or_signature() {
    let token = seal(&key('a'), "test", &"hello", 1_000);
    let (payload, tag) = token.split_once('.').unwrap();
    let longer = seal(&key('a'), "test", &"hello", 9_000);
    let forged = format!("{}.{tag}", longer.split_once('.').unwrap().0);
    assert_eq!(open::<String>(&key('a'), "test", &forged, at(0)), None);
    let flipped = format!("{payload}.{}", tag.replacen(&tag[..1], "A", 1));
    assert_eq!(open::<String>(&key('a'), "test", &flipped, at(0)), None);
    for junk in ["", ".", "a.b", "no-dot", &format!("{payload}.")] {
        assert_eq!(open::<String>(&key('a'), "test", junk, at(0)), None);
    }
}

#[test]
fn random_values_differ() {
    assert_ne!(random(), random());
    assert_eq!(random().len(), 22);
}
