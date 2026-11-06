use super::*;

const SECRET: &str = "made-up-secret-made-up-secret-0001";
const OTHER: &str = "another-made-up-secret-0000000002";
const ID: &str = "123456789012345678";

fn sealer(secret: &str) -> Sealer {
    Sealer::new(&KeySecret::parse(secret).unwrap())
}

#[test]
fn opens_what_it_sealed() {
    let sealer = sealer(SECRET);
    let sealed = sealer.seal(ID, "app1");
    assert_eq!(sealer.open(&sealed, "app1").as_deref(), Some(ID));
}

#[test]
fn the_sealed_value_hides_the_id_and_differs_each_time() {
    let sealer = sealer(SECRET);
    let (a, b) = (sealer.seal(ID, "app1"), sealer.seal(ID, "app1"));
    assert!(a.starts_with("v1."));
    assert!(!a.contains(ID));
    assert_ne!(a, b);
}

#[test]
fn another_key_or_record_cannot_open_it() {
    let sealed = sealer(SECRET).seal(ID, "app1");
    assert_eq!(sealer(OTHER).open(&sealed, "app1"), None);
    assert_eq!(sealer(SECRET).open(&sealed, "app2"), None);
}

#[test]
fn altered_or_plain_values_are_refused() {
    let sealer = sealer(SECRET);
    let sealed = sealer.seal(ID, "app1");
    let mut altered = sealed.clone();
    altered.pop();
    altered.push(if sealed.ends_with('A') { 'B' } else { 'A' });
    assert_eq!(sealer.open(&altered, "app1"), None);
    assert_eq!(sealer.open(ID, "app1"), None);
    assert_eq!(sealer.open("v1.", "app1"), None);
    assert_eq!(sealer.open("v1.!!", "app1"), None);
}

#[test]
fn opens_a_value_sealed_by_the_dev_tool() {
    // Made by `sealDiscordId` in the dev tool, with an all-sevens nonce.
    let sealed = "v1.BwcHBwcHBwcHBwcH8Cr6BdoU7zrvdNXvmWgh98anHrkquMvdsW_lDmmIQM6R0w";
    assert_eq!(sealer(SECRET).open(sealed, "app1").as_deref(), Some(ID));
}
