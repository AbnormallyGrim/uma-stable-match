//! Trading an authorization code for the signed-in user's Discord ID.

use reqwest::header::USER_AGENT;
use serde::Deserialize;

use crate::config::DiscordConfig;
use crate::error::Error;
use crate::rest::{api_error, BOT_USER_AGENT};

#[derive(Deserialize)]
struct Token {
    access_token: String,
}

#[derive(Deserialize)]
struct User {
    id: String,
}

/// The Discord ID of the user who approved `code`. The access token lives only inside this call.
pub(crate) async fn signed_in_user(
    http: &reqwest::Client,
    config: &DiscordConfig,
    code: &str,
    redirect_uri: &str,
) -> Result<u64, Error> {
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
    user.id
        .parse()
        .map_err(|_| Error::Decode(serde::de::Error::custom("the user ID isn't a whole number")))
}

async fn read<T: serde::de::DeserializeOwned>(response: reqwest::Response) -> Result<T, Error> {
    let status = response.status();
    let bytes = response.bytes().await?;
    if !status.is_success() {
        return Err(api_error(status, &bytes));
    }
    serde_json::from_slice(&bytes).map_err(Error::Decode)
}
