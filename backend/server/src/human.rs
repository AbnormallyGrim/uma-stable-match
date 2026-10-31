//! Proof that a visitor is a person: a Cloudflare Turnstile check, exchanged for a short-lived
//! signed token. See the README's Human check section.

use std::fmt;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::extract::{Request, State};
use axum::http::header::{AUTHORIZATION, COOKIE, SET_COOKIE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::Sha256;

use crate::api_error::ApiError;
use crate::app::AppState;

/// How long a token stays valid.
pub const TOKEN_LIFETIME: Duration = Duration::from_secs(10 * 60);

/// The cookie that carries the token between page loads.
pub const TOKEN_COOKIE: &str = "usm_human";

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
    site_key: String,
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
    /// `site_key` is Turnstile's public key, which the web app needs to show the challenge.
    pub fn new(site_key: impl Into<String>, secret: TurnstileSecret, key: SigningKey) -> Self {
        let client = reqwest::Client::builder()
            .timeout(VERIFY_TIMEOUT)
            .build()
            .expect("the HTTP client builds with default settings");
        Self {
            enabled: Some(Arc::new(Enabled {
                site_key: site_key.into(),
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
            site_key: enabled.site_key.clone(),
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

/// Turns away requests without a valid token, in an `Authorization: Bearer` header or the token
/// cookie. Lets everything through when the check is off.
pub(crate) async fn require_human(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let headers = request.headers();
    let passed = !state.human.is_enabled()
        || bearer(headers).is_some_and(|token| state.human.is_valid(token))
        || cookies(headers).any(|token| state.human.is_valid(token));
    if passed {
        return next.run(request).await;
    }
    metrics::counter!("human_checks_total", "result" => "token_missing").increment(1);
    ApiError::new(
        StatusCode::UNAUTHORIZED,
        "a human check token is required; get one from /api/v1/challenge",
    )
    .into_response()
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
}

// Every value of the token cookie, since a browser can send more than one.
fn cookies(headers: &HeaderMap) -> impl Iterator<Item = &str> {
    headers
        .get_all(COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .filter(|(name, _)| *name == TOKEN_COOKIE)
        .map(|(_, value)| value)
}

fn token_cookie(token: &str) -> HeaderValue {
    let max_age = TOKEN_LIFETIME.as_secs();
    HeaderValue::from_str(&format!(
        "{TOKEN_COOKIE}={token}; Max-Age={max_age}; Path=/api; HttpOnly; Secure; SameSite=Strict"
    ))
    .expect("a signed token is a valid header value")
}

/// Serves `GET /api/v1/challenge`: Turnstile's public site key, for the web app.
pub(crate) async fn site_key(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    match &state.human.enabled {
        Some(enabled) => Ok(Json(json!({ "site_key": enabled.site_key }))),
        None => Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "the human check is off",
        )),
    }
}

/// Serves `POST /api/v1/challenge`: trades a Turnstile response for a token, also set as a cookie.
pub(crate) async fn challenge(
    State(state): State<AppState>,
    Json(body): Json<Challenge>,
) -> Result<Response, ApiError> {
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
            let token = sign(&enabled.key, SystemTime::now());
            let cookie = token_cookie(&token);
            let body = Json(Token {
                token,
                expires_in: TOKEN_LIFETIME.as_secs(),
            });
            Ok(([(SET_COOKIE, cookie)], body).into_response())
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
