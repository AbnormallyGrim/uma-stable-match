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
    name: String,
    permissions: String,
}

#[derive(Deserialize)]
pub(crate) struct Member {
    roles: Vec<String>,
}

/// A role in a server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NamedRole {
    pub(crate) id: u64,
    pub(crate) name: String,
}

/// What the bot learned about a server while checking someone's rights in it.
#[derive(Debug)]
pub(crate) struct Access {
    /// The server's roles, without `@everyone`.
    pub(crate) roles: Vec<NamedRole>,
}

/// The server's roles if `user_id` owns it or has Administrator or Manage Server there, as the bot
/// sees it; `None` if not. Someone who isn't a member has no rights. A server the bot can't see is
/// an error.
pub(crate) async fn verify(
    rest: &Rest,
    guild_id: u64,
    user_id: u64,
) -> Result<Option<Access>, Error> {
    let guild: Guild = rest.get(&format!("/guilds/{guild_id}")).await?;
    let member = match rest
        .get::<Member>(&format!("/guilds/{guild_id}/members/{user_id}"))
        .await
    {
        Ok(member) => member,
        Err(error) if error.status() == Some(crate::StatusCode::NOT_FOUND) => return Ok(None),
        Err(error) => return Err(error),
    };
    if !has_manage(&guild, guild_id, user_id, &member) {
        return Ok(None);
    }
    let mut roles: Vec<NamedRole> = guild
        .roles
        .iter()
        .filter(|role| role.id != guild_id.to_string())
        .filter_map(|role| {
            Some(NamedRole {
                id: role.id.parse().ok()?,
                name: role.name.clone(),
            })
        })
        .collect();
    roles.sort_by_key(|role| role.name.to_lowercase());
    Ok(Some(Access { roles }))
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
