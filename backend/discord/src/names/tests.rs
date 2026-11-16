use super::is_avatar_hash;

#[test]
fn avatar_hashes_are_32_hex_digits_maybe_animated() {
    assert!(is_avatar_hash("0123456789abcdef0123456789abcdef"));
    assert!(is_avatar_hash("a_0123456789abcdef0123456789abcdef"));
}

#[test]
fn anything_else_is_not_an_avatar_hash() {
    assert!(!is_avatar_hash(""));
    assert!(!is_avatar_hash("0123456789ABCDEF0123456789ABCDEF"));
    assert!(!is_avatar_hash("0123456789abcdef"));
    assert!(!is_avatar_hash("../../0123456789abcdef0123456789"));
    assert!(!is_avatar_hash("b_0123456789abcdef0123456789abcdef"));
}
