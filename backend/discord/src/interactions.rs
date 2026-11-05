//! The endpoint Discord sends interactions to, such as button clicks.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use ed25519_dalek::Signature;
use serde::Deserialize;
use serde_json::json;

use crate::auth::error;
use crate::bot::Discord;
use crate::config::{decode_hex, PublicKey};

/// Path of the interactions endpoint, to set as the Interactions Endpoint URL in Discord.
pub const INTERACTIONS_PATH: &str = "/discord/interactions";

/// Largest gap accepted between a signature's timestamp and now, to limit replays.
pub const MAX_TIMESTAMP_SKEW: Duration = Duration::from_secs(5 * 60);

/// Reply to unhandled interactions.
const UNSUPPORTED: &str = "This action isn't available yet.";

const PING: u8 = 1;
const PONG: u8 = 1;
const CHANNEL_MESSAGE: u8 = 4;
const EPHEMERAL: u32 = 1 << 6;

/// Serves `POST /discord/interactions` and the sign-in routes.
pub fn router(discord: Discord) -> Router {
    Router::new()
        .route(INTERACTIONS_PATH, post(interactions))
        .with_state(discord.clone())
        .merge(crate::auth::router(discord))
}

#[derive(Deserialize)]
struct Interaction {
    #[serde(rename = "type")]
    kind: u8,
}

async fn interactions(State(discord): State<Discord>, headers: HeaderMap, body: Bytes) -> Response {
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
    let Ok(interaction) = serde_json::from_slice::<Interaction>(&body) else {
        return error(StatusCode::BAD_REQUEST, "invalid interaction");
    };
    match interaction.kind {
        PING => Json(json!({ "type": PONG })).into_response(),
        _ => Json(json!({
            "type": CHANNEL_MESSAGE,
            "data": { "content": UNSUPPORTED, "flags": EPHEMERAL },
        }))
        .into_response(),
    }
}

/// Whether `signature` (hex) signs `timestamp` followed by `body` with `key`, and `timestamp`
/// (Unix seconds) is within [`MAX_TIMESTAMP_SKEW`] of `now`.
pub(crate) fn verify(
    key: &PublicKey,
    signature: &str,
    timestamp: &str,
    body: &[u8],
    now: SystemTime,
) -> bool {
    let Ok(seconds) = timestamp.parse::<u64>() else {
        return false;
    };
    let now = now.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    if now.abs_diff(seconds) > MAX_TIMESTAMP_SKEW.as_secs() {
        return false;
    }
    let Some(signature) = decode_hex(signature).and_then(|b| Signature::from_slice(&b).ok()) else {
        return false;
    };
    let mut message = Vec::with_capacity(timestamp.len() + body.len());
    message.extend_from_slice(timestamp.as_bytes());
    message.extend_from_slice(body);
    key.key().verify_strict(&message, &signature).is_ok()
}

#[cfg(test)]
mod tests;
