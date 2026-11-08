//! Sign in with Discord: the OAuth2 redirect and callback, and the session they start.

use std::time::{Duration, SystemTime};

use axum::extract::{Query, State};
use axum::http::header::{ACCEPT, CACHE_CONTROL, COOKIE, ORIGIN, SET_COOKIE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::bot::Discord;
use crate::config::DiscordConfig;
use crate::opt_in::store_unavailable;
use crate::store::StoreError;
use crate::token::{open, open_expiring, random, seal, seconds};

/// Path that sends the visitor to Discord to sign in.
pub const LOGIN_PATH: &str = "/api/v1/auth/discord/login";

/// Path Discord sends the visitor back to. Set `<public URL>` plus this as a Redirect in the
/// Discord Developer Portal.
pub const CALLBACK_PATH: &str = "/api/v1/auth/discord/callback";

/// Path that answers with who is signed in.
pub const SESSION_PATH: &str = "/api/v1/auth/session";

/// Path that ends the session.
pub const SIGN_OUT_PATH: &str = "/api/v1/auth/sign-out";

/// Path that deletes what's kept about the signed-in person and ends the session.
pub const ACCOUNT_PATH: &str = "/api/v1/auth/account";

/// How long a sign-in link is good for, from the redirect to Discord until the callback.
pub const MAX_STATE_AGE: Duration = Duration::from_secs(10 * 60);

/// How long a session lasts.
pub const MAX_SESSION_AGE: Duration = Duration::from_secs(12 * 60 * 60);

const STATE_PURPOSE: &str = "state";
const SESSION_PURPOSE: &str = "session";
const NONCE_COOKIE: &str = "uma_discord_nonce";
const SESSION_COOKIE: &str = "uma_session";
/// Cookie the frontend can read, holding the session's expiry, so pages know a session exists
/// without asking. It holds nothing secret and proves nothing; the session cookie does.
pub const SIGNED_IN_COOKIE: &str = "uma_signed_in";
/// Longest `next` path kept.
const MAX_NEXT_LEN: usize = 200;

/// What a sign-in is for, which sets the scopes asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Flow {
    /// Signing in to use the applications pages.
    SignIn,
    /// A club leader adding the bot to a server.
    OptIn,
    /// A player applying to a club.
    Apply,
}

impl Flow {
    fn parse(name: &str) -> Option<Self> {
        match name {
            "sign_in" => Some(Self::SignIn),
            "opt_in" => Some(Self::OptIn),
            "apply" => Some(Self::Apply),
            _ => None,
        }
    }

    /// The OAuth2 scopes to ask for.
    pub(crate) fn scopes(self) -> &'static str {
        match self {
            Self::SignIn => "identify",
            Self::OptIn => "bot identify",
            Self::Apply => "identify guilds.join",
        }
    }

    /// Whether the callback can finish this flow yet.
    fn available(self) -> bool {
        self == Self::SignIn
    }
}

/// What the signed `state` carries.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct OAuthState {
    f: Flow,
    /// Must match the nonce cookie.
    n: String,
    /// Where to send the visitor afterwards.
    r: String,
    /// For opting a club in: what the install callback must find.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) t: Option<Target>,
}

/// Where a browser is sent when a sign-in or opt-in fails, with `?error=` and the failure's label.
pub const APPLICATIONS_PAGE: &str = "/applications";

/// Whether the request is a page load, which asks for HTML.
pub(crate) fn wants_html(headers: &HeaderMap) -> bool {
    headers
        .get(ACCEPT)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|accept| accept.contains("text/html"))
}

/// What an opt-in is meant to do, signed into the `state`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Target {
    /// The club to link.
    pub(crate) club: u64,
    /// The server the club's invite points to, which the bot must be added to.
    pub(crate) guild: u64,
    /// The signed-in user who started it.
    pub(crate) user: u64,
}

/// Serves the sign-in routes.
pub(crate) fn router(discord: Discord) -> Router {
    Router::new()
        .route(LOGIN_PATH, get(login))
        .route(CALLBACK_PATH, get(callback))
        .route(SESSION_PATH, get(session_info))
        .route(SIGN_OUT_PATH, post(sign_out))
        .route(ACCOUNT_PATH, delete(delete_account))
        .with_state(discord.clone())
        .merge(crate::trainers::router(discord.clone()))
        .merge(crate::apply::router(discord.clone()))
        .merge(crate::opt_in::router(discord))
}

#[derive(Deserialize)]
struct LoginParams {
    flow: Option<String>,
    next: Option<String>,
}

#[derive(Deserialize)]
struct CallbackParams {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

async fn login(State(discord): State<Discord>, Query(params): Query<LoginParams>) -> Response {
    let flow = params
        .flow
        .as_deref()
        .map_or(Some(Flow::SignIn), Flow::parse);
    let Some(flow) = flow.filter(|flow| flow.available()) else {
        return error(StatusCode::NOT_FOUND, "that sign-in isn't available");
    };
    let config = discord.config();
    let next = safe_next(params.next.as_deref().unwrap_or("/"));
    let (state, nonce) = begin(config, flow, next, None, SystemTime::now());
    send_to_discord(config, flow, &state, &nonce, &[("prompt", "none")])
}

/// Redirects to Discord's authorize page for `flow`, with the nonce set as a cookie. `extra` adds
/// to the query.
pub(crate) fn send_to_discord(
    config: &DiscordConfig,
    flow: Flow,
    state: &str,
    nonce: &str,
    extra: &[(&str, &str)],
) -> Response {
    let mut url = reqwest::Url::parse(&config.authorize_url).expect("the authorize URL is valid");
    {
        let mut query = url.query_pairs_mut();
        query
            .append_pair("client_id", &config.app_id.get().to_string())
            .append_pair("redirect_uri", &redirect_uri(config))
            .append_pair("response_type", "code")
            .append_pair("scope", flow.scopes())
            .append_pair("state", state);
        for (name, value) in extra {
            query.append_pair(name, value);
        }
    }
    let cookie = set_cookie(
        config,
        NONCE_COOKIE,
        nonce,
        "/api/v1/auth/discord",
        MAX_STATE_AGE,
    );
    ([(SET_COOKIE, cookie)], Redirect::temporary(url.as_str())).into_response()
}

async fn callback(
    State(discord): State<Discord>,
    Query(params): Query<CallbackParams>,
    headers: HeaderMap,
) -> Response {
    let config = discord.config();
    // Whatever happens, the nonce is used up.
    let clear = set_cookie(
        config,
        NONCE_COOKIE,
        "",
        "/api/v1/auth/discord",
        Duration::ZERO,
    );
    let outcome = finish(&discord, &params, &headers).await;
    let (result, mut response) = match outcome {
        Ok((id, next)) => {
            let expires = seconds(SystemTime::now() + MAX_SESSION_AGE);
            let session = set_cookie(
                config,
                SESSION_COOKIE,
                &seal(
                    &config.signing_key,
                    SESSION_PURPOSE,
                    &id.to_string(),
                    expires,
                ),
                "/",
                MAX_SESSION_AGE,
            );
            let marker = signed_in_cookie(config, &expires.to_string(), MAX_SESSION_AGE);
            let mut response = Redirect::temporary(&next).into_response();
            response.headers_mut().append(SET_COOKIE, session);
            response.headers_mut().append(SET_COOKIE, marker);
            ("success", response)
        }
        // A browser lands on the applications page, which explains what went wrong; anything else
        // gets the JSON error.
        Err(failure) if wants_html(&headers) => (
            failure.label,
            Redirect::to(&format!("{APPLICATIONS_PAGE}?error={}", failure.label)).into_response(),
        ),
        Err(failure) => (failure.label, error(failure.status, failure.message)),
    };
    metrics::counter!("discord_sign_ins_total", "result" => result).increment(1);
    response.headers_mut().append(SET_COOKIE, clear);
    response
}

pub(crate) struct Failure {
    pub(crate) label: &'static str,
    status: StatusCode,
    message: &'static str,
}

impl Failure {
    /// What to send for this failure: a browser lands on the applications page, which explains
    /// it, and anything else gets the JSON error.
    pub(crate) fn respond(self, headers: &HeaderMap) -> Response {
        if wants_html(headers) {
            Redirect::to(&format!("{APPLICATIONS_PAGE}?error={}", self.label)).into_response()
        } else {
            error(self.status, self.message)
        }
    }
}

impl Failure {
    pub(crate) fn new(label: &'static str, status: StatusCode, message: &'static str) -> Self {
        Self {
            label,
            status,
            message,
        }
    }
}

/// Checks the callback and trades its code for the user's ID and where to send them next.
async fn finish(
    discord: &Discord,
    params: &CallbackParams,
    headers: &HeaderMap,
) -> Result<(u64, String), Failure> {
    let config = discord.config();
    let invalid = || {
        Failure::new(
            "invalid_state",
            StatusCode::BAD_REQUEST,
            "this sign-in link is invalid or has expired; start again",
        )
    };
    let state = params.state.as_deref().ok_or_else(invalid)?;
    let state = check_state(
        config,
        state,
        cookie_values(headers, NONCE_COOKIE),
        SystemTime::now(),
    )
    .ok_or_else(invalid)?;
    if params.error.is_some() {
        return Err(Failure::new(
            "denied",
            StatusCode::BAD_REQUEST,
            "sign-in was cancelled",
        ));
    }
    let code = params
        .code
        .as_deref()
        .filter(|code| !code.is_empty())
        .ok_or_else(invalid)?;
    let authorized = crate::oauth::authorize(discord.http(), config, code, &redirect_uri(config))
        .await
        .map_err(|err| {
            tracing::warn!(error = %err, "Discord sign-in failed");
            Failure::new(
                "discord_error",
                StatusCode::BAD_GATEWAY,
                "Discord didn't accept the sign-in; try again",
            )
        })?;
    match (state.f, state.t) {
        (Flow::SignIn, _) => Ok((authorized.user_id, state.r)),
        (Flow::OptIn, Some(target)) => {
            crate::opt_in::install(discord, target, &authorized).await?;
            Ok((authorized.user_id, state.r))
        }
        (Flow::Apply, Some(target)) => {
            crate::apply::submit(discord, target, &authorized).await?;
            Ok((authorized.user_id, state.r))
        }
        _ => Err(invalid()),
    }
}

#[derive(Serialize)]
struct Me {
    discord_id: String,
    /// `None` when Discord couldn't be asked.
    name: Option<String>,
}

/// The request's signed, unexpired session: its token, Discord ID and expiry (Unix seconds).
fn signed_session<'a>(
    config: &DiscordConfig,
    headers: &'a HeaderMap,
) -> Option<(&'a str, u64, u64)> {
    cookie_values(headers, SESSION_COOKIE).find_map(|token| {
        let (id, expires) = open_expiring::<String>(
            &config.signing_key,
            SESSION_PURPOSE,
            token,
            SystemTime::now(),
        )?;
        Some((token, id.parse().ok()?, expires))
    })
}

/// The Discord ID in the request's session, if it has a valid one that hasn't been ended.
pub(crate) async fn session_user(
    discord: &Discord,
    headers: &HeaderMap,
) -> Result<Option<u64>, StoreError> {
    let Some((token, user, expires)) = signed_session(discord.config(), headers) else {
        return Ok(None);
    };
    let started = expires.saturating_sub(MAX_SESSION_AGE.as_secs());
    match discord.sessions() {
        Some(sessions) if !sessions.valid(token, user, started).await? => Ok(None),
        _ => Ok(Some(user)),
    }
}

/// The signed-in user, or the response to send: a 401 without a session, a 503 if it can't be
/// checked.
pub(crate) async fn signed_in(
    discord: &Discord,
    headers: &HeaderMap,
) -> Result<u64, Box<Response>> {
    match session_user(discord, headers).await {
        Ok(Some(user)) => Ok(user),
        Ok(None) => Err(Box::new(error(
            StatusCode::UNAUTHORIZED,
            "sign in with Discord first",
        ))),
        Err(err) => Err(Box::new(store_unavailable(&err))),
    }
}

async fn session_info(State(discord): State<Discord>, headers: HeaderMap) -> Response {
    let id = match session_user(&discord, &headers).await {
        Ok(Some(id)) => id,
        Ok(None) => {
            let mut response = private(error(StatusCode::UNAUTHORIZED, "not signed in"));
            let marker = signed_in_cookie(discord.config(), "", Duration::ZERO);
            response.headers_mut().append(SET_COOKIE, marker);
            return response;
        }
        Err(err) => return private(store_unavailable(&err)),
    };
    let name = discord.names().get(discord.rest(), id).await;
    let mut response = private(Json(Me {
        discord_id: id.to_string(),
        name,
    }));
    // Puts the marker back if it went missing while the session still holds.
    if let Some((_, _, expires)) = signed_session(discord.config(), &headers) {
        let left = expires.saturating_sub(seconds(SystemTime::now()));
        let marker = signed_in_cookie(
            discord.config(),
            &expires.to_string(),
            Duration::from_secs(left),
        );
        response.headers_mut().append(SET_COOKIE, marker);
    }
    response
}

/// Ends the session, so a copy of the cookie stops working too. Refused from another site.
async fn sign_out(State(discord): State<Discord>, headers: HeaderMap) -> Response {
    let config = discord.config();
    if !from_this_site(config, &headers) {
        return error(StatusCode::FORBIDDEN, "sign out from this site");
    }
    if let (Some(sessions), Some((token, _, expires))) =
        (discord.sessions(), signed_session(config, &headers))
    {
        let now = seconds(SystemTime::now());
        if let Err(err) = sessions.revoke(token, expires, now).await {
            return store_unavailable(&err);
        }
    }
    signed_out(&discord)
}

/// Whether the request's `Origin`, if it has one, is this site's scheme and host. Browsers send it
/// with every `POST`; the port is ignored, so a local dev server's proxy still matches.
fn from_this_site(config: &DiscordConfig, headers: &HeaderMap) -> bool {
    let Some(origin) = headers.get(ORIGIN) else {
        return true;
    };
    let site = reqwest::Url::parse(config.public_url.as_str()).expect("the public URL is valid");
    origin
        .to_str()
        .ok()
        .and_then(|origin| reqwest::Url::parse(origin).ok())
        .is_some_and(|origin| origin.scheme() == site.scheme() && origin.host() == site.host())
}

/// Deletes the person's trainer links and pending keys, ends all their sessions and signs them
/// out.
async fn delete_account(State(discord): State<Discord>, headers: HeaderMap) -> Response {
    let user = match signed_in(&discord, &headers).await {
        Ok(user) => user,
        Err(response) => return *response,
    };
    if let Some(links) = discord.trainer_links() {
        if let Err(err) = links.store().forget(user).await {
            return store_unavailable(&err);
        }
    }
    if let Some(sessions) = discord.sessions() {
        let now = seconds(SystemTime::now());
        let mut ended = sessions.end_all(user, now).await;
        // This one too, since a session started in the same second is kept.
        if let (Ok(()), Some((token, _, expires))) =
            (&ended, signed_session(discord.config(), &headers))
        {
            ended = sessions.revoke(token, expires, now).await;
        }
        if let Err(err) = ended {
            return store_unavailable(&err);
        }
    }
    discord.names().forget(user);
    metrics::counter!("discord_account_deletions_total").increment(1);
    signed_out(&discord)
}

/// A 204 that clears the session.
fn signed_out(discord: &Discord) -> Response {
    let session = set_cookie(discord.config(), SESSION_COOKIE, "", "/", Duration::ZERO);
    let marker = signed_in_cookie(discord.config(), "", Duration::ZERO);
    let mut response = StatusCode::NO_CONTENT.into_response();
    response.headers_mut().append(SET_COOKIE, session);
    response.headers_mut().append(SET_COOKIE, marker);
    response
}

/// The `state` to send to Discord and the nonce to set as a cookie, for a sign-in starting `now`.
pub(crate) fn begin(
    config: &DiscordConfig,
    flow: Flow,
    next: &str,
    target: Option<Target>,
    now: SystemTime,
) -> (String, String) {
    let nonce = random();
    let state = OAuthState {
        f: flow,
        n: nonce.clone(),
        r: next.to_owned(),
        t: target,
    };
    let token = seal(
        &config.signing_key,
        STATE_PURPOSE,
        &state,
        seconds(now + MAX_STATE_AGE),
    );
    (token, nonce)
}

/// The `state` if it's signed, unexpired at `now`, and carries the nonce one of `nonces` holds.
pub(crate) fn check_state<'a>(
    config: &DiscordConfig,
    state: &str,
    mut nonces: impl Iterator<Item = &'a str>,
    now: SystemTime,
) -> Option<OAuthState> {
    let state: OAuthState = open(&config.signing_key, STATE_PURPOSE, state, now)?;
    nonces.any(|nonce| nonce == state.n).then_some(state)
}

/// `next` if it's a path on this site, else `/`.
pub(crate) fn safe_next(next: &str) -> &str {
    let local = next.starts_with('/')
        && !next.starts_with("//")
        && next.len() <= MAX_NEXT_LEN
        && !next.chars().any(|c| c == '\\' || c.is_control());
    if local {
        next
    } else {
        "/"
    }
}

pub(crate) fn redirect_uri(config: &DiscordConfig) -> String {
    format!("{}{CALLBACK_PATH}", config.public_url.as_str())
}

pub(crate) fn set_cookie(
    config: &DiscordConfig,
    name: &str,
    value: &str,
    path: &str,
    max_age: Duration,
) -> HeaderValue {
    cookie(config, name, value, path, max_age, "; HttpOnly")
}

/// The [`SIGNED_IN_COOKIE`], readable by the page, set to `value` for `max_age`.
fn signed_in_cookie(config: &DiscordConfig, value: &str, max_age: Duration) -> HeaderValue {
    cookie(config, SIGNED_IN_COOKIE, value, "/", max_age, "")
}

fn cookie(
    config: &DiscordConfig,
    name: &str,
    value: &str,
    path: &str,
    max_age: Duration,
    http_only: &str,
) -> HeaderValue {
    let secure = if config.public_url.is_https() {
        "; Secure"
    } else {
        ""
    };
    HeaderValue::from_str(&format!(
        "{name}={value}; Path={path}; Max-Age={}{http_only}; SameSite=Lax{secure}",
        max_age.as_secs()
    ))
    .expect("cookie values are visible ASCII")
}

/// Every value of the cookie `name`, since a browser can send more than one.
pub(crate) fn cookie_values<'a>(
    headers: &'a HeaderMap,
    name: &'a str,
) -> impl Iterator<Item = &'a str> {
    headers
        .get_all(COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .filter(move |(cookie, _)| *cookie == name)
        .map(|(_, value)| value)
}

pub(crate) fn error(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({ "error": message }))).into_response()
}

/// `body` with `Cache-Control: no-store`, for responses that hold a person's data.
pub(crate) fn private(body: impl IntoResponse) -> Response {
    let mut response = body.into_response();
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

#[cfg(test)]
mod tests;
