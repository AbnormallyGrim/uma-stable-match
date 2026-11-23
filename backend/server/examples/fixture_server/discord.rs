//! A mock Discord for the fixture server: the sign-in page, the OAuth2 token exchange and the REST
//! calls the club applications make, plus a control route that sets trainers' profile comments.
//! Every user manages every server, and club `n`'s invite leads to server `GUILD_BASE + n`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{delete, get, post, put};
use axum::{Form, Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use uma_stable_match_discord::DiscordConfig;

use crate::Comments;

/// Server ID of club `n` is `GUILD_BASE + n`.
const GUILD_BASE: u64 = 700_000_000_000_000_000;
/// Discord's Administrator permission, held by everyone on the mock servers.
const ADMINISTRATOR: &str = "8";
/// Discord's channel type for a category.
const CATEGORY: u8 = 4;

/// The invite code in club `n`'s description.
pub fn invite(n: u64) -> String {
    format!("fixture-club-{n}")
}

#[derive(Clone)]
struct Mock {
    /// Where sign-ins may return to: the server's callback.
    redirect_uri: String,
    comments: Comments,
    /// Next ID for a created channel or message.
    next_id: Arc<AtomicU64>,
}

/// Serves the mock on `127.0.0.1:port` and points `config` at it.
pub async fn start(
    port: u16,
    config: &mut DiscordConfig,
    comments: Comments,
) -> std::io::Result<()> {
    let base = format!("http://127.0.0.1:{port}");
    config.api_base = format!("{base}/api");
    config.authorize_url = format!("{base}/oauth2/authorize");
    let mock = Mock {
        redirect_uri: format!(
            "{}{}",
            config.public_url.as_str(),
            uma_stable_match_discord::CALLBACK_PATH
        ),
        comments,
        next_id: Arc::new(AtomicU64::new(GUILD_BASE * 2)),
    };
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    tokio::spawn(async move {
        if let Err(error) = axum::serve(listener, router(mock)).await {
            tracing::error!("the mock Discord stopped: {error}");
        }
    });
    Ok(())
}

fn router(mock: Mock) -> Router {
    Router::new()
        .route("/oauth2/authorize", get(authorize))
        .route("/oauth2/approve", get(approve))
        .route("/fixture/profiles/{trainer}/comment", put(set_comment))
        .route("/api/oauth2/token", post(token))
        .route("/api/users/{user}", get(user))
        .route("/api/users/@me/guilds/{guild}", delete(no_content))
        .route("/api/invites/{code}", get(invite_guild))
        .route("/api/guilds/{guild}", get(guild))
        .route("/api/guilds/{guild}/members/{user}", get(member).put(join))
        .route("/api/guilds/{guild}/channels", get(channels).post(create))
        .route(
            "/api/channels/{channel}",
            axum::routing::patch(create).delete(create),
        )
        .route("/api/channels/{channel}/messages", post(create))
        .route(
            "/api/channels/{channel}/messages/{message}",
            axum::routing::patch(create).delete(no_content),
        )
        .fallback(unknown)
        .with_state(mock)
}

#[derive(Deserialize)]
struct AuthorizeParams {
    redirect_uri: String,
    state: String,
    guild_id: Option<String>,
}

/// Discord's consent page: asks which Discord user ID to sign in as.
async fn authorize(Query(params): Query<AuthorizeParams>) -> Html<String> {
    let guild = params.guild_id.unwrap_or_default();
    Html(format!(
        r#"<!doctype html><html lang="en"><head><title>Mock Discord</title></head><body>
<main><h1>Mock Discord</h1>
<form action="/oauth2/approve" method="get">
<input type="hidden" name="redirect_uri" value="{}">
<input type="hidden" name="state" value="{}">
<input type="hidden" name="guild_id" value="{}">
<label>Discord user ID <input name="user" inputmode="numeric" required></label>
<button type="submit">Authorize</button>
<button type="submit" name="deny" value="1" formnovalidate>Cancel</button>
</form></main></body></html>"#,
        escape(&params.redirect_uri),
        escape(&params.state),
        escape(&guild),
    ))
}

#[derive(Deserialize)]
struct ApproveParams {
    redirect_uri: String,
    state: String,
    guild_id: Option<String>,
    user: Option<String>,
    deny: Option<String>,
}

/// Sends the browser back to the server, with a code that names the user and server.
async fn approve(State(mock): State<Mock>, Query(params): Query<ApproveParams>) -> Response {
    if params.redirect_uri != mock.redirect_uri {
        return (StatusCode::BAD_REQUEST, "unknown redirect_uri").into_response();
    }
    let state = urlencode(&params.state);
    if params.deny.is_some() {
        return Redirect::to(&format!(
            "{}?error=access_denied&state={state}",
            mock.redirect_uri
        ))
        .into_response();
    }
    let Some(user) = params
        .user
        .as_deref()
        .and_then(|u| u.trim().parse::<u64>().ok())
    else {
        return (StatusCode::BAD_REQUEST, "user must be a Discord ID").into_response();
    };
    let guild = params
        .guild_id
        .as_deref()
        .and_then(|g| g.parse::<u64>().ok())
        .unwrap_or(0);
    Redirect::to(&format!(
        "{}?code={user}-{guild}&state={state}",
        mock.redirect_uri
    ))
    .into_response()
}

#[derive(Deserialize)]
struct TokenForm {
    code: String,
}

async fn token(Form(form): Form<TokenForm>) -> Response {
    let Some((user, guild)) = form
        .code
        .split_once('-')
        .and_then(|(u, g)| Some((u.parse::<u64>().ok()?, g.parse::<u64>().ok()?)))
    else {
        return discord_error(StatusCode::BAD_REQUEST, 0, "invalid_grant");
    };
    let guild = (guild != 0).then(|| json!({ "id": guild.to_string() }));
    Json(json!({
        "access_token": format!("fixture-token-{user}"),
        "token_type": "Bearer",
        "guild": guild,
    }))
    .into_response()
}

/// A user by ID, or `@me` for the holder of a user's access token.
async fn user(Path(user): Path<String>, headers: HeaderMap) -> Response {
    let id = if user == "@me" {
        headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer fixture-token-"))
            .map(str::to_owned)
    } else {
        Some(user)
    };
    match id.and_then(|id| id.parse::<u64>().ok()) {
        Some(id) => Json(json!({
            "id": id.to_string(),
            "username": format!("fixture_user_{id}"),
            "global_name": format!("Fixture User {id}"),
            "avatar": null,
        }))
        .into_response(),
        None => discord_error(StatusCode::UNAUTHORIZED, 0, "401: Unauthorized"),
    }
}

async fn invite_guild(Path(code): Path<String>) -> Response {
    let club = code
        .strip_prefix("fixture-club-")
        .and_then(|n| n.parse::<u64>().ok());
    match club {
        Some(n) => Json(json!({ "code": code, "guild": { "id": (GUILD_BASE + n).to_string() } }))
            .into_response(),
        None => discord_error(StatusCode::NOT_FOUND, 10006, "Unknown Invite"),
    }
}

async fn guild(Path(guild): Path<u64>) -> Json<Value> {
    Json(json!({
        "id": guild.to_string(),
        "name": format!("Fixture Server {}", guild.saturating_sub(GUILD_BASE)),
        "owner_id": "1",
        "roles": [{ "id": guild.to_string(), "name": "@everyone", "permissions": ADMINISTRATOR }],
    }))
}

async fn member(Path((_, user)): Path<(u64, u64)>) -> Json<Value> {
    Json(json!({ "user": { "id": user.to_string() }, "roles": [] }))
}

async fn join(Path((_, user)): Path<(u64, u64)>) -> (StatusCode, Json<Value>) {
    (
        StatusCode::CREATED,
        Json(json!({ "user": { "id": user.to_string() }, "roles": [] })),
    )
}

async fn channels(Path(guild): Path<u64>) -> Json<Value> {
    Json(json!([
        { "id": (guild + 1).to_string(), "name": "Applications", "type": CATEGORY },
        { "id": (guild + 2).to_string(), "name": "general", "type": 0 },
    ]))
}

/// Creating, changing or deleting a channel or message: answers with a new ID.
async fn create(State(mock): State<Mock>) -> Json<Value> {
    let id = mock.next_id.fetch_add(1, Ordering::Relaxed);
    Json(json!({ "id": id.to_string() }))
}

async fn no_content() -> StatusCode {
    StatusCode::NO_CONTENT
}

/// Sets the profile comment the fixture source returns for `trainer`.
async fn set_comment(
    State(mock): State<Mock>,
    Path(trainer): Path<u64>,
    body: Bytes,
) -> StatusCode {
    let comment = String::from_utf8_lossy(&body).into_owned();
    match mock.comments.lock() {
        Ok(mut comments) => {
            comments.insert(trainer, comment);
            StatusCode::NO_CONTENT
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn unknown(method: axum::http::Method, uri: axum::http::Uri) -> Response {
    tracing::warn!(%method, %uri, "the mock Discord has no such route");
    discord_error(StatusCode::NOT_FOUND, 0, "404: Not Found")
}

fn discord_error(status: StatusCode, code: u32, message: &str) -> Response {
    (status, Json(json!({ "code": code, "message": message }))).into_response()
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn urlencode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.~".contains(&byte) {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}
