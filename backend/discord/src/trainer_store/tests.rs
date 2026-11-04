use super::*;

fn secret(value: &str) -> KeySecret {
    KeySecret::parse(value).unwrap()
}

#[test]
fn the_hash_is_stable_and_keyed() {
    let one = secret("made-up-secret-made-up-secret-0001");
    let two = secret("made-up-secret-made-up-secret-0002");
    assert_eq!(applicant_hash(&one, 7), applicant_hash(&one, 7));
    assert_ne!(applicant_hash(&one, 7), applicant_hash(&one, 8));
    assert_ne!(applicant_hash(&one, 7), applicant_hash(&two, 7));
}

#[test]
fn the_hash_is_hex_and_hides_the_id() {
    let hash = applicant_hash(&secret("made-up-secret-made-up-secret-0001"), 123456789012);
    assert_eq!(hash.len(), 32);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    assert!(!hash.contains("123456789012"));
}

#[test]
fn a_short_secret_is_refused_and_hidden() {
    assert!(KeySecret::parse("too-short").is_none());
    let shown = format!("{:?}", secret("made-up-secret-made-up-secret-0001"));
    assert!(!shown.contains("made-up"));
}
