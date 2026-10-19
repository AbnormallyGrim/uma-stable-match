use super::*;

#[test]
fn paths_with_file_extensions() {
    assert!(has_extension("/assets/index-abc123.js"));
    assert!(has_extension("/favicon.ico"));
    assert!(has_extension("/theme-init.js"));
    assert!(!has_extension("/"));
    assert!(!has_extension("/results"));
    assert!(!has_extension("/some/page"));
    assert!(!has_extension("/v1.2/page/"));
}
