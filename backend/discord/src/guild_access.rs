//! Whether someone manages a Discord server: owns it, or has Administrator or Manage Server. That's
//! what Discord asks of anyone adding a bot, and the opt-in checks it again before linking.

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
