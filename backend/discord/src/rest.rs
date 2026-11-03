//! Discord's REST API, called as the bot.

use std::sync::Arc;

use reqwest::header::{AUTHORIZATION, USER_AGENT};
use reqwest::{Method, StatusCode};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

use crate::config::{BotToken, DiscordConfig, RateLimitConfig};
use crate::error::Error;
use crate::rate_limit::{retry_after, route_key, RateLimits};

/// The user agent Discord asks bots to send.
const BOT_USER_AGENT: &str = concat!(
    "DiscordBot (https://github.com/AbnormallyGrim/uma-stable-match, ",
    env!("CARGO_PKG_VERSION"),
    ")"
);

/// A client for Discord's REST API that waits out its rate limits. Cheap to clone.
#[derive(Debug, Clone)]
pub struct Rest {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    http: reqwest::Client,
    base: String,
    token: BotToken,
    rate_limit: RateLimitConfig,
    limits: RateLimits,
}

impl Rest {
    /// A client with `config`'s token, base URL and rate-limit settings.
    pub fn new(config: &DiscordConfig) -> Result<Self, Error> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
        Ok(Self {
            inner: Arc::new(Inner {
                http,
                base: config.api_base.trim_end_matches('/').to_owned(),
                token: config.bot_token.clone(),
                rate_limit: config.rate_limit,
                limits: RateLimits::default(),
            }),
        })
    }

    /// `GET path`, such as `/guilds/123`.
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, Error> {
        self.send(Method::GET, path, None).await
    }

    /// `POST path` with a JSON body.
    pub async fn post<T: DeserializeOwned>(
        &self,
        path: &str,
        body: &impl Serialize,
    ) -> Result<T, Error> {
        self.send(Method::POST, path, Some(to_value(body)?)).await
    }

    /// `PUT path` with a JSON body.
    pub async fn put<T: DeserializeOwned>(
        &self,
        path: &str,
        body: &impl Serialize,
    ) -> Result<T, Error> {
        self.send(Method::PUT, path, Some(to_value(body)?)).await
    }

    /// `PATCH path` with a JSON body.
    pub async fn patch<T: DeserializeOwned>(
        &self,
        path: &str,
        body: &impl Serialize,
    ) -> Result<T, Error> {
        self.send(Method::PATCH, path, Some(to_value(body)?)).await
    }

    /// `DELETE path`.
    pub async fn delete<T: DeserializeOwned>(&self, path: &str) -> Result<T, Error> {
        self.send(Method::DELETE, path, None).await
    }

    /// Sends a request, waiting out rate limits and retrying 429s. An empty response body reads
    /// as JSON `null`, so `()` works for 204 responses.
    async fn send<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<T, Error> {
        let inner = &self.inner;
        let route = route_key(&method, path);
        let mut retries = 0;
        loop {
            inner.limits.wait(&route, inner.rate_limit.max_wait).await?;
            let mut request = inner
                .http
                .request(method.clone(), format!("{}{path}", inner.base))
                .header(AUTHORIZATION, format!("Bot {}", inner.token.as_str()))
                .header(USER_AGENT, BOT_USER_AGENT);
            if let Some(body) = &body {
                request = request.json(body);
            }
            let response = request.send().await?;
            let status = response.status();
            metrics::counter!("discord_requests_total", "status" => status.as_u16().to_string())
                .increment(1);
            let headers = response.headers().clone();
            inner.limits.update(&route, &headers);
            let bytes = response.bytes().await?;

            if status == StatusCode::TOO_MANY_REQUESTS {
                let (wait, global) = retry_after(&headers, &bytes);
                inner.limits.limited(&route, wait, global);
                if retries >= inner.rate_limit.max_retries || wait > inner.rate_limit.max_wait {
                    return Err(Error::RateLimited(wait));
                }
                retries += 1;
                tracing::warn!(
                    route = %route,
                    retry_after = wait.as_secs_f64(),
                    global,
                    "Discord rate limited a request"
                );
                continue;
            }
            if !status.is_success() {
                return Err(api_error(status, &bytes));
            }
            let json: &[u8] = if bytes.is_empty() { b"null" } else { &bytes };
            return serde_json::from_slice(json).map_err(Error::Decode);
        }
    }
}

fn to_value(body: &impl Serialize) -> Result<Value, Error> {
    serde_json::to_value(body).map_err(Error::Decode)
}

fn api_error(status: StatusCode, body: &[u8]) -> Error {
    #[derive(serde::Deserialize)]
    struct Body {
        code: Option<u32>,
        message: Option<String>,
    }
    let body = serde_json::from_slice::<Body>(body).ok();
    Error::Api {
        status,
        code: body.as_ref().and_then(|b| b.code),
        message: body
            .and_then(|b| b.message)
            .unwrap_or_else(|| status.canonical_reason().unwrap_or("error").to_owned()),
    }
}
