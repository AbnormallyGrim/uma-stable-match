//! Trading an authorization code for the signed-in user's Discord ID.

use reqwest::header::USER_AGENT;
use serde::Deserialize;

use crate::config::DiscordConfig;
use crate::error::Error;
use crate::rest::{api_error, BOT_USER_AGENT};

#[derive(Deserialize)]
struct Token {
    access_token: String,
    /// Present when the bot scope was granted: the server the bot was added to.
    guild: Option<Guild>,
}

#[derive(Deserialize)]
struct Guild {
    id: String,
}

/// Who approved a code, and the server the bot was added to if it was.
pub(crate) struct Authorized {
    pub(crate) user_id: u64,
    pub(crate) guild_id: Option<u64>,
}

#[derive(Deserialize)]
struct User {
    id: String,
}

/// Who approved `code`, and the server it added the bot to. The access token lives only inside
/// this call.
pub(crate) async fn authorize(
    http: &reqwest::Client,
    config: &DiscordConfig,
    code: &str,
    redirect_uri: &str,
) -> Result<Authorized, Error> {
    let base = config.api_base.trim_end_matches('/');
    let app_id = config.app_id.get().to_string();
    let response = http
        .post(format!("{base}/oauth2/token"))
        .header(USER_AGENT, BOT_USER_AGENT)
        .form(&[
            ("client_id", app_id.as_str()),
            ("client_secret", config.client_secret.as_str()),
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
        ])
        .send()
        .await?;
    let token: Token = read(response).await?;

    let response = http
        .get(format!("{base}/users/@me"))
        .header(USER_AGENT, BOT_USER_AGENT)
        .bearer_auth(&token.access_token)
        .send()
        .await?;
    let user: User = read(response).await?;
    let id = |id: &str| {
        id.parse::<u64>()
            .map_err(|_| Error::Decode(serde::de::Error::custom("an ID isn't a whole number")))
    };
    Ok(Authorized {
        user_id: id(&user.id)?,
        guild_id: token
            .guild
            .as_ref()
            .map(|guild| id(&guild.id))
            .transpose()?,
    })
}

async fn read<T: serde::de::DeserializeOwned>(response: reqwest::Response) -> Result<T, Error> {
    let status = response.status();
    let bytes = response.bytes().await?;
    if !status.is_success() {
        return Err(api_error(status, &bytes));
    }
    serde_json::from_slice(&bytes).map_err(Error::Decode)
}
