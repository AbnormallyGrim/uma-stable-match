//! Whether someone manages a Discord server, which is what lets them unlink a club from it.

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

/// Whether `user_id` owns the server or has Administrator or Manage Server there, as the bot sees
/// it. Someone who isn't a member doesn't. A server the bot can't see is an error.
pub(crate) async fn manages(rest: &Rest, guild_id: u64, user_id: u64) -> Result<bool, Error> {
    let guild: Guild = rest.get(&format!("/guilds/{guild_id}")).await?;
    match rest
        .get::<Member>(&format!("/guilds/{guild_id}/members/{user_id}"))
        .await
    {
        Ok(member) => Ok(has_manage(&guild, guild_id, user_id, &member)),
        Err(error) if error.status() == Some(crate::StatusCode::NOT_FOUND) => Ok(false),
        Err(error) => Err(error),
    }
}

/// Whether the owner, or a member whose `@everyone` role (the one with the server's ID) or other
/// roles grant Administrator or Manage Server.
pub(crate) fn has_manage(guild: &Guild, guild_id: u64, user_id: u64, member: &Member) -> bool {
    if guild.owner_id == user_id.to_string() {
        return true;
    }
    let everyone = guild_id.to_string();
    let granted = guild
        .roles
        .iter()
        .filter(|role| role.id == everyone || member.roles.contains(&role.id))
        .filter_map(|role| role.permissions.parse::<u64>().ok())
        .fold(0, |all, permissions| all | permissions);
    granted & (ADMINISTRATOR | MANAGE_GUILD) != 0
}

#[cfg(test)]
mod tests;
