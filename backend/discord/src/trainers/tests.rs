use super::*;

#[test]
fn keys_have_the_prefix_and_only_clear_characters() {
    for _ in 0..200 {
        let key = new_key();
        let code = key.strip_prefix("uma-").expect("the prefix");
        assert_eq!(code.len(), KEY_CHARS);
        assert!(code.bytes().all(|b| ALPHABET.contains(&b)), "{key}");
    }
}

#[test]
fn keys_differ() {
    let keys: std::collections::HashSet<String> = (0..50).map(|_| new_key()).collect();
    assert!(keys.len() > 45);
}

#[test]
fn the_alphabet_leaves_out_lookalikes() {
    for c in b"ILO01" {
        assert!(!ALPHABET.contains(c));
    }
}

#[test]
fn finds_the_key_anywhere_in_the_comment_ignoring_case() {
    assert!(contains_key("uma-7KQ4XM", "uma-7KQ4XM"));
    assert!(contains_key("Hi! UMA-7kq4xm :)", "uma-7KQ4XM"));
    assert!(contains_key("xxuma-7KQ4XMxx", "uma-7KQ4XM"));
    assert!(!contains_key("uma-7KQ4XN", "uma-7KQ4XM"));
    assert!(!contains_key("", "uma-7KQ4XM"));
    assert!(!contains_key("uma 7KQ4XM", "uma-7KQ4XM"));
}

#[test]
fn trainer_ids_are_nine_to_twelve_digits() {
    assert_eq!(parse_trainer("100000001"), Some(100_000_001));
    assert_eq!(parse_trainer("123456789012"), Some(123_456_789_012));
    for bad in [
        "",
        "12345678",
        "1234567890123",
        "+12345678901",
        "12345678a",
        "１２３４５６７８９",
    ] {
        assert_eq!(parse_trainer(bad), None, "{bad}");
    }
}
