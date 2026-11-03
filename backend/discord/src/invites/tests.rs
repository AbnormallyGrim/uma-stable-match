use super::*;

#[test]
fn finds_links_in_every_form() {
    let text = "Join https://discord.gg/abc123 or discord.com/invite/Xy-9z, \
                or http://www.discordapp.com/invite/Qq77Qq77!";
    assert_eq!(
        invite_candidates(text),
        ["abc123", "Xy-9z", "Qq77Qq77"].map(String::from)
    );
}

#[test]
fn links_are_case_insensitive_but_codes_keep_their_case() {
    assert_eq!(
        invite_candidates("DISCORD.GG/AbCd"),
        ["AbCd"].map(String::from)
    );
}

#[test]
fn finds_a_bare_code_after_the_word_discord() {
    for text in [
        "Discord: aB3dEfG",
        "discord - 8xk2mQ",
        "Our Discord invite: aB3dEfG!",
        "DISCORD code=aB3dEfG",
        "discord aB3dEfG and more",
    ] {
        assert_eq!(invite_candidates(text).len(), 1, "{text}");
    }
    assert_eq!(
        invite_candidates("Discord: aB3dEfG"),
        ["aB3dEfG"].map(String::from)
    );
}

#[test]
fn ordinary_words_after_discord_are_not_codes() {
    for text in [
        "Discord server active",
        "Discord required",
        "We use Discord Everyday",
        "discordapp is great",
        "Discord: ",
        "Discord",
        "Join our Discord, it's fun",
        "discord.gg",
    ] {
        assert!(invite_candidates(text).is_empty(), "{text}");
    }
}

#[test]
fn bare_codes_need_sensible_lengths() {
    assert!(invite_candidates("discord a1b2").is_empty());
    assert!(invite_candidates("discord a1b2c3d4e5f6g7").is_empty());
}

#[test]
fn skips_repeats_and_keeps_the_order_they_appear() {
    let text = "discord.gg/bbb discord.gg/aaa discord.gg/bbb Discord: bbb";
    assert_eq!(invite_candidates(text), ["bbb", "aaa"].map(String::from));
}

#[test]
fn keeps_at_most_five() {
    let text: String = (0..9).map(|i| format!("discord.gg/code{i} ")).collect();
    let found = invite_candidates(&text);
    assert_eq!(found.len(), MAX_INVITE_CANDIDATES);
    assert_eq!(found[0], "code0");
}

#[test]
fn ignores_links_that_arent_invites() {
    for text in [
        "https://notdiscord.gg/abc",
        "discord.gg/",
        "discord.gg/ x",
        "discord.com/channels/1/2",
        &format!("discord.gg/{}", "a".repeat(40)),
        "",
    ] {
        assert!(invite_candidates(text).is_empty(), "{text}");
    }
}

#[test]
fn handles_non_ascii_text() {
    assert_eq!(
        invite_candidates("ウマ娘 クラブ discord.gg/abc123 ✨ Discord：xY9zQw"),
        ["abc123"].map(String::from)
    );
    assert!(invite_candidates("ディスコード discord.gg/").is_empty());
}
