//! Proof that a visitor is a person: a Cloudflare Turnstile check, exchanged for a short-lived
//! signed token. See the README's Human check section.

use std::fmt;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::api_error::ApiError;
use crate::app::AppState;

/// How long a token stays valid.
pub const TOKEN_LIFETIME: Duration = Duration::from_secs(10 * 60);

const VERIFY_URL: &str = "https://challenges.cloudflare.com/turnstile/v0/siteverify";
const VERIFY_TIMEOUT: Duration = Duration::from_secs(5);
const MIN_KEY_LEN: usize = 32;
const MAX_RESPONSE_LEN: usize = 2048;

/// Turnstile's secret key. `Debug` output hides it.
#[derive(Clone, PartialEq, Eq)]
pub struct TurnstileSecret(String);

impl TurnstileSecret {
    /// The secret, or `None` if it's blank.
    pub fn parse(value: &str) -> Option<Self> {
        let value = value.trim();
        (!value.is_empty()).then(|| Self(value.to_owned()))
    }
}

impl fmt::Debug for TurnstileSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("TurnstileSecret(<redacted>)")
    }
}

/// Key that signs tokens. `Debug` output hides it.
#[derive(Clone, PartialEq, Eq)]
pub struct SigningKey(String);

impl SigningKey {
    /// The key, or `None` if it's shorter than 32 characters.
    pub fn parse(value: &str) -> Option<Self> {
        (value.chars().count() >= MIN_KEY_LEN).then(|| Self(value.to_owned()))
    }
}

impl fmt::Debug for SigningKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SigningKey(<redacted>)")
    }
}

/// The human check. Off by default, when every request passes.
#[derive(Debug, Clone, Default)]
pub struct HumanCheck {
    enabled: Option<Arc<Enabled>>,
}

#[derive(Debug)]
struct Enabled {
    secret: TurnstileSecret,
    key: SigningKey,
    client: reqwest::Client,
    verify_url: String,
}

#[derive(Serialize)]
struct VerifyRequest<'a> {
    secret: &'a str,
    response: &'a str,
}

#[derive(Deserialize)]
struct VerifyReply {
    success: bool,
}

enum VerifyError {
    Failed,
    Unreachable,
}

impl HumanCheck {
    /// A check that signs tokens with `key` after Turnstile accepts a response made with `secret`.
    pub fn new(secret: TurnstileSecret, key: SigningKey) -> Self {
        let client = reqwest::Client::builder()
            .timeout(VERIFY_TIMEOUT)
            .build()
            .expect("the HTTP client builds with default settings");
        Self {
            enabled: Some(Arc::new(Enabled {
                secret,
                key,
                client,
                verify_url: VERIFY_URL.to_owned(),
            })),
        }
    }

    /// Verifies against `url` instead of Cloudflare, for tests. No effect on an off check.
    pub fn with_verify_url(self, url: impl Into<String>) -> Self {
        let Some(enabled) = self.enabled else {
            return self;
        };
        let enabled = Arc::new(Enabled {
            secret: enabled.secret.clone(),
            key: enabled.key.clone(),
            client: enabled.client.clone(),
            verify_url: url.into(),
        });
        Self {
            enabled: Some(enabled),
        }
    }

    /// Whether requests need a token.
    pub fn is_enabled(&self) -> bool {
        self.enabled.is_some()
    }

    /// Whether `token` was signed by this server and hasn't expired. Always true when off.
    pub fn is_valid(&self, token: &str) -> bool {
        let Some(enabled) = &self.enabled else {
            return true;
        };
        valid_at(&enabled.key, token, SystemTime::now())
    }

    async fn verify(&self, enabled: &Enabled, response: &str) -> Result<(), VerifyError> {
        let reply = enabled
            .client
            .post(&enabled.verify_url)
            .json(&VerifyRequest {
                secret: &enabled.secret.0,
                response,
            })
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|_| VerifyError::Unreachable)?
            .json::<VerifyReply>()
            .await
            .map_err(|_| VerifyError::Unreachable)?;
        if reply.success {
            Ok(())
        } else {
            Err(VerifyError::Failed)
        }
    }
}

fn mac(key: &SigningKey, expires: u64) -> Hmac<Sha256> {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(key.0.as_bytes()).expect("HMAC accepts any key length");
    mac.update(format!("human.v1.{expires}").as_bytes());
    mac
}

fn seconds(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

fn sign(key: &SigningKey, now: SystemTime) -> String {
    let expires = seconds(now) + TOKEN_LIFETIME.as_secs();
    let tag: String = mac(key, expires)
        .finalize()
        .into_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    format!("{expires}.{tag}")
}

fn valid_at(key: &SigningKey, token: &str, now: SystemTime) -> bool {
    let Some((expires, tag)) = token.split_once('.') else {
        return false;
    };
    let Ok(expires) = expires.parse::<u64>() else {
        return false;
    };
    if expires <= seconds(now) || tag.len() % 2 != 0 {
        return false;
    }
    let bytes: Option<Vec<u8>> = (0..tag.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(tag.get(i..i + 2)?, 16).ok())
        .collect();
    bytes.is_some_and(|bytes| mac(key, expires).verify_slice(&bytes).is_ok())
}

#[derive(Deserialize)]
pub(crate) struct Challenge {
    response: String,
}

#[derive(Serialize)]
pub(crate) struct Token {
    token: String,
    expires_in: u64,
}

/// Serves `POST /api/v1/challenge`: trades a Turnstile response for a token.
pub(crate) async fn challenge(
    State(state): State<AppState>,
    Json(body): Json<Challenge>,
) -> Result<Json<Token>, ApiError> {
    let Some(enabled) = &state.human.enabled else {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "the human check is off",
        ));
    };
    if body.response.is_empty() || body.response.len() > MAX_RESPONSE_LEN {
        return Err(ApiError::bad_request("response is missing or too long"));
    }
    match state.human.verify(enabled, &body.response).await {
        Ok(()) => {
            metrics::counter!("human_checks_total", "result" => "passed").increment(1);
            Ok(Json(Token {
                token: sign(&enabled.key, SystemTime::now()),
                expires_in: TOKEN_LIFETIME.as_secs(),
            }))
        }
        Err(VerifyError::Failed) => {
            metrics::counter!("human_checks_total", "result" => "failed").increment(1);
            Err(ApiError::new(
                StatusCode::FORBIDDEN,
                "the human check failed; try again",
            ))
        }
        Err(VerifyError::Unreachable) => {
            metrics::counter!("human_checks_total", "result" => "unavailable").increment(1);
            Err(ApiError::new(
                StatusCode::BAD_GATEWAY,
                "the human check is unavailable; try again later",
            ))
        }
    }
}

#[cfg(test)]
mod tests;
