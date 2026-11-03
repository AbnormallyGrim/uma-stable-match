//! Finding a club's Discord server through the invites in its description.

use crate::error::Error;
use crate::rest::Rest;

/// Most invite candidates tried for one club.
pub const MAX_INVITE_CANDIDATES: usize = 5;

/// Link prefixes, lowercase, that an invite code follows.
const LINK_PREFIXES: [&str; 3] = [
    "discord.gg/",
    "discord.com/invite/",
    "discordapp.com/invite/",
];
/// Words that may sit between "discord" and a bare code, such as "Discord invite: aB3dEfG".
const FILLER_WORDS: [&str; 3] = ["invite", "code", "link"];
const MAX_CODE_LEN: usize = 32;
const MIN_BARE_LEN: usize = 6;
const MAX_BARE_LEN: usize = 12;

/// Invite codes in `description`, in the order they appear, without repeats and at most
/// [`MAX_INVITE_CANDIDATES`].
///
/// They are guesses: a code is only an invite once Discord resolves it. Links
/// (`discord.gg/…`, `discord.com/invite/…`, `discordapp.com/invite/…`) count as they are; a bare
/// code after the word "discord" must also look random (a digit, or capitals inside it), so
/// ordinary words don't count.
pub fn invite_candidates(description: &str) -> Vec<String> {
    let lower = description.to_ascii_lowercase();
    let mut found: Vec<(usize, String)> = Vec::new();

    for prefix in LINK_PREFIXES {
        for (at, _) in lower.match_indices(prefix) {
            if lower[..at]
                .chars()
                .next_back()
                .is_some_and(char::is_alphanumeric)
            {
                continue;
            }
            let start = at + prefix.len();
            let code = code_at(description, start);
            if (2..=MAX_CODE_LEN).contains(&code.len()) {
                found.push((at, code.to_owned()));
            }
        }
    }
    for (at, _) in lower.match_indices("discord") {
        if lower[..at]
            .chars()
            .next_back()
            .is_some_and(char::is_alphanumeric)
        {
            continue;
        }
        let mut rest = skip_separators(&description[at + "discord".len()..]);
        // "discord.gg" and "discordapp" are links or other words, not the word on its own.
        if rest.len() == description.len() - at - "discord".len() && !rest.is_empty() {
            continue;
        }
        let lowered = rest.to_ascii_lowercase();
        if let Some(word) = FILLER_WORDS.iter().find(|word| lowered.starts_with(**word)) {
            rest = skip_separators(&rest[word.len()..]);
        }
        let code = alphanumeric_prefix(rest);
        if (MIN_BARE_LEN..=MAX_BARE_LEN).contains(&code.len())
            && looks_random(code)
            && rest[code.len()..]
                .chars()
                .next()
                .is_none_or(|c| !c.is_alphanumeric() && c != '-')
        {
            found.push((at, code.to_owned()));
        }
    }

    found.sort_by_key(|(at, _)| *at);
    let mut codes: Vec<String> = Vec::new();
    for (_, code) in found {
        if !codes.contains(&code) {
            codes.push(code);
        }
    }
    codes.truncate(MAX_INVITE_CANDIDATES);
    codes
}

/// The server ID an invite points to, or `None` if Discord doesn't know the invite (expired,
/// deleted or never real) or it isn't for a server. Other failures are errors, so a Discord outage
/// isn't read as a missing invite.
pub async fn resolve_invite(rest: &Rest, code: &str) -> Result<Option<u64>, Error> {
    #[derive(serde::Deserialize)]
    struct Invite {
        guild: Option<Guild>,
    }
    #[derive(serde::Deserialize)]
    struct Guild {
        id: String,
    }
    // Candidates only ever hold letters, digits and hyphens, so the code can't change the path.
    match rest.get::<Invite>(&format!("/invites/{code}")).await {
        Ok(invite) => Ok(invite.guild.and_then(|guild| guild.id.parse().ok())),
        Err(error) if error.status() == Some(crate::StatusCode::NOT_FOUND) => Ok(None),
        Err(error) => Err(error),
    }
}

/// The servers the invites in `description` point to, in the order they appear and without
/// repeats. Invites Discord doesn't know are skipped. Empty means the club points to no server.
pub async fn servers_in_description(rest: &Rest, description: &str) -> Result<Vec<u64>, Error> {
    let mut servers = Vec::new();
    for code in invite_candidates(description) {
        if let Some(server) = resolve_invite(rest, &code).await? {
            if !servers.contains(&server) {
                servers.push(server);
            }
        }
    }
    Ok(servers)
}

/// The invite code starting at byte `start`: letters, digits and hyphens.
fn code_at(text: &str, start: usize) -> &str {
    let rest = &text[start..];
    let end = rest
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
        .unwrap_or(rest.len());
    &rest[..end]
}

fn alphanumeric_prefix(text: &str) -> &str {
    let end = text
        .find(|c: char| !c.is_ascii_alphanumeric())
        .unwrap_or(text.len());
    &text[..end]
}

fn skip_separators(text: &str) -> &str {
    text.trim_start_matches(|c: char| {
        c.is_whitespace() || matches!(c, ':' | '=' | '-' | '–' | '—' | '|' | '>' | '*')
    })
}

/// Whether a bare code could be a real one: a digit, or capitals after the first letter.
fn looks_random(code: &str) -> bool {
    code.chars().any(|c| c.is_ascii_digit())
        || (code.chars().any(|c| c.is_ascii_lowercase())
            && code.chars().skip(1).any(|c| c.is_ascii_uppercase()))
}

#[cfg(test)]
mod tests;
