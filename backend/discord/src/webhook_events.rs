//! The endpoint Discord sends webhook events to. Only removing the app is acted on: the person's
//! sessions end on every device.

use std::time::SystemTime;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::Router;
use serde::Deserialize;

use crate::auth::error;
use crate::bot::Discord;
use crate::interactions::verify;
use crate::opt_in::store_unavailable;
use crate::token::seconds;

/// Path of the webhook events endpoint, to set as the Webhook Events URL in Discord.
pub const WEBHOOK_EVENTS_PATH: &str = "/discord/webhook-events";

const PING: u8 = 0;
const EVENT: u8 = 1;
const DEAUTHORIZED: &str = "APPLICATION_DEAUTHORIZED";

pub(crate) fn router(discord: Discord) -> Router {
    Router::new()
        .route(WEBHOOK_EVENTS_PATH, post(events))
        .with_state(discord)
}

#[derive(Deserialize)]
struct Payload {
    #[serde(rename = "type")]
    kind: u8,
    event: Option<Event>,
}

#[derive(Deserialize)]
struct Event {
    #[serde(rename = "type")]
    kind: String,
    data: Option<Data>,
}

#[derive(Deserialize)]
struct Data {
    user: Option<User>,
}

#[derive(Deserialize)]
struct User {
    id: String,
}

async fn events(State(discord): State<Discord>, headers: HeaderMap, body: Bytes) -> Response {
    let header = |name| headers.get(name).and_then(|v| v.to_str().ok());
    let verified = match (
        header("x-signature-ed25519"),
        header("x-signature-timestamp"),
    ) {
        (Some(signature), Some(timestamp)) => verify(
            &discord.config().public_key,
            signature,
            timestamp,
            &body,
            SystemTime::now(),
        ),
        _ => false,
    };
    if !verified {
        return error(StatusCode::UNAUTHORIZED, "invalid request signature");
    }
    let Ok(payload) = serde_json::from_slice::<Payload>(&body) else {
        return error(StatusCode::BAD_REQUEST, "invalid webhook event");
    };
    match (payload.kind, payload.event) {
        (PING, _) => StatusCode::NO_CONTENT.into_response(),
        (EVENT, Some(event)) if event.kind == DEAUTHORIZED => deauthorized(&discord, event).await,
        // Other events aren't used; acknowledged so Discord doesn't retry them.
        _ => StatusCode::NO_CONTENT.into_response(),
    }
}

/// Ends every session of the person who removed the app.
async fn deauthorized(discord: &Discord, event: Event) -> Response {
    let Some(user) = event
        .data
        .and_then(|data| data.user)
        .and_then(|user| user.id.parse::<u64>().ok())
    else {
        return error(StatusCode::BAD_REQUEST, "the event names no user");
    };
    if let Some(sessions) = discord.sessions() {
        // A failure answers 503, so Discord tries again.
        if let Err(err) = sessions.end_all(user, seconds(SystemTime::now())).await {
            return store_unavailable(&err);
        }
    }
    discord.names().forget(user);
    metrics::counter!("discord_deauthorizations_total").increment(1);
    StatusCode::NO_CONTENT.into_response()
}
