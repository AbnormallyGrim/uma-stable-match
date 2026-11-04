//! What someone can do in a Discord server: manage it, which lets them change and unlink a club,
//! or hold a club's leader role, which lets them see its applications.

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

/// How far someone can go with a club.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Level {
    /// Read the club's applications, as a holder of one of its leader roles.
    View,
    /// Also change its settings, as someone who manages the server.
    Manage,
}

impl Level {
    /// The word the API uses for the level.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::View => "view",
            Self::Manage => "manage",
        }
    }
}

/// What a member of a server can do there, as the bot sees it.
#[derive(Debug)]
pub(crate) struct Rights {
    /// Whether they own the server or have Administrator or Manage Server.
    pub(crate) manage: bool,
    /// The server's roles they hold.
    held: Vec<u64>,
    /// The server's roles, without `@everyone`.
    pub(crate) roles: Vec<NamedRole>,
}

impl Rights {
    /// What these rights allow for a club whose leader roles are `leader_roles`: everything for
    /// someone who manages the server, viewing for a holder of a leader role, else nothing.
    pub(crate) fn level(&self, leader_roles: &[u64]) -> Option<Level> {
        if self.manage {
            Some(Level::Manage)
        } else if leader_roles.iter().any(|role| self.held.contains(role)) {
            Some(Level::View)
        } else {
            None
        }
    }
}

/// What `user_id` can do in the server, or `None` if they aren't a member. A server the bot can't
/// see is an error.
pub(crate) async fn rights(
    rest: &Rest,
    guild_id: u64,
    user_id: u64,
) -> Result<Option<Rights>, Error> {
    let guild: Guild = rest.get(&format!("/guilds/{guild_id}")).await?;
    let member = match rest
        .get::<Member>(&format!("/guilds/{guild_id}/members/{user_id}"))
        .await
    {
        Ok(member) => member,
        Err(error) if error.status() == Some(crate::StatusCode::NOT_FOUND) => return Ok(None),
        Err(error) => return Err(error),
    };
    let manage = has_manage(&guild, guild_id, user_id, &member);
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
    let held = member.roles.iter().filter_map(|r| r.parse().ok()).collect();
    Ok(Some(Rights {
        manage,
        held,
        roles,
    }))
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
