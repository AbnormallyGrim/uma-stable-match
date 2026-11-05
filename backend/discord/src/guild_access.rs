//! Whether someone manages a Discord server: owns it, or has Administrator or Manage Server. That's
//! what Discord asks of anyone adding a bot, and the opt-in checks it again before linking. Also
//! which of the permissions the bot needs it was left without.

use serde::Deserialize;

use crate::error::Error;
use crate::rest::Rest;

const ADMINISTRATOR: u64 = 1 << 3;
const MANAGE_GUILD: u64 = 1 << 5;

#[derive(Deserialize)]
pub(crate) struct Guild {
    owner_id: String,
    roles: Vec<Role>,
}

#[derive(Deserialize)]
struct Role {
    id: String,
    permissions: String,
}

#[derive(Deserialize)]
pub(crate) struct Member {
    roles: Vec<String>,
}

/// Whether `user_id` manages the server; `false` if they aren't a member. A server the bot can't
/// see is an error.
pub(crate) async fn manages(rest: &Rest, guild_id: u64, user_id: u64) -> Result<bool, Error> {
    let guild: Guild = rest.get(&format!("/guilds/{guild_id}")).await?;
    let member = match rest
        .get::<Member>(&format!("/guilds/{guild_id}/members/{user_id}"))
        .await
    {
        Ok(member) => member,
        Err(error) if error.status() == Some(crate::StatusCode::NOT_FOUND) => return Ok(false),
        Err(error) => return Err(error),
    };
    Ok(has_manage(&guild, guild_id, user_id, &member))
}

/// Whether the owner, or a member whose `@everyone` role (the one with the server's ID) or other
/// roles grant Administrator or Manage Server.
pub(crate) fn has_manage(guild: &Guild, guild_id: u64, user_id: u64, member: &Member) -> bool {
    if guild.owner_id == user_id.to_string() {
        return true;
    }
    granted(guild, guild_id, member) & (ADMINISTRATOR | MANAGE_GUILD) != 0
}

/// The bits of `wanted` the bot is missing in the server, `0` if it has them all. The bot is the
/// member with the application's ID. Channel overrides aren't looked at.
pub(crate) async fn missing_for_bot(
    rest: &Rest,
    guild_id: u64,
    bot_id: u64,
    wanted: u64,
) -> Result<u64, Error> {
    let guild: Guild = rest.get(&format!("/guilds/{guild_id}")).await?;
    let member: Member = rest
        .get(&format!("/guilds/{guild_id}/members/{bot_id}"))
        .await?;
    Ok(missing(&guild, guild_id, &member, wanted))
}

/// The bits of `wanted` that the member's roles don't grant. Administrator grants everything.
pub(crate) fn missing(guild: &Guild, guild_id: u64, member: &Member, wanted: u64) -> u64 {
    let granted = granted(guild, guild_id, member);
    if granted & ADMINISTRATOR != 0 {
        return 0;
    }
    wanted & !granted
}

/// The permissions of `@everyone` and the member's roles, combined.
fn granted(guild: &Guild, guild_id: u64, member: &Member) -> u64 {
    let everyone = guild_id.to_string();
    guild
        .roles
        .iter()
        .filter(|role| role.id == everyone || member.roles.contains(&role.id))
        .filter_map(|role| role.permissions.parse::<u64>().ok())
        .fold(0, |all, permissions| all | permissions)
}

#[cfg(test)]
mod tests;
