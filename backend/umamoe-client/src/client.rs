//! The API client.

use std::fmt;
use std::sync::Arc;
use std::time::Instant;

use reqwest::header::{HeaderValue, ACCEPT};
use reqwest::Url;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::config::{ClientConfig, RetryConfig};
use crate::error::{ApiErrorBody, Error};
use crate::rate_limit::{Priority, RateLimiter};
use crate::requests::{CircleRequest, ListCirclesRequest, RankingsGainsRequest};
use crate::retry;
use crate::types::{CircleListResponse, CircleResponse, GainsRankingsResponse};

const API_KEY_HEADER: &str = "X-API-Key";

/// uma.moe API client. Clones share one connection pool and rate limit.
#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    base_url: String,
    api_key: HeaderValue,
    retry: RetryConfig,
    limiter: Arc<RateLimiter>,
    priority: Priority,
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client")
            .field("base_url", &self.base_url)
            .field("priority", &self.priority)
            .finish_non_exhaustive()
    }
}

impl Client {
    /// Builds a client from settings.
    pub fn new(config: ClientConfig) -> Result<Self, Error> {
        let base_url = config.base_url.trim_end_matches('/').to_owned();
        Url::parse(&base_url).map_err(|e| Error::Config(format!("base URL: {e}")))?;

        let mut api_key = HeaderValue::from_str(config.api_key.expose())
            .map_err(|_| Error::Config("API key contains invalid characters".into()))?;
        api_key.set_sensitive(true);

        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .user_agent(config.user_agent)
            .build()
            .map_err(|e| Error::Config(e.to_string()))?;

        Ok(Self {
            http,
            base_url,
            api_key,
            retry: config.retry,
            limiter: Arc::new(RateLimiter::new(config.min_interval)),
            priority: Priority::default(),
        })
    }

    /// A clone whose requests wait in another lane. It shares this client's rate limit.
    pub fn with_priority(&self, priority: Priority) -> Self {
        Self {
            priority,
            ..self.clone()
        }
    }

    /// Fetches a club and its members' fan data (`GET /api/v4/circles`).
    pub async fn circle(&self, request: &CircleRequest) -> Result<CircleResponse, Error> {
        if let Some((_, month)) = request.period {
            if !(1..=12).contains(&month) {
                return Err(Error::InvalidRequest(format!(
                    "month must be 1 to 12, got {month}"
                )));
            }
        }
        self.get("/api/v4/circles", &request.query()).await
    }

    /// Fetches one page of clubs (`GET /api/v4/circles/list`).
    pub async fn list_circles(
        &self,
        request: &ListCirclesRequest,
    ) -> Result<CircleListResponse, Error> {
        self.get("/api/v4/circles/list", request).await
    }

    /// Fetches one page of rolling fan gains (`GET /api/v4/rankings/gains`).
    pub async fn rankings_gains(
        &self,
        request: &RankingsGainsRequest,
    ) -> Result<GainsRankingsResponse, Error> {
        self.get("/api/v4/rankings/gains", request).await
    }

    async fn get<T, Q>(&self, path: &str, query: &Q) -> Result<T, Error>
    where
        T: DeserializeOwned,
        Q: Serialize + ?Sized,
    {
        let url = format!("{}{path}", self.base_url);
        let lane = lane_label(self.priority);
        let mut retries = 0;
        loop {
            let waiting = Instant::now();
            self.limiter.acquire(self.priority).await;
            metrics::histogram!("umamoe_queue_wait_seconds", "lane" => lane)
                .record(waiting.elapsed().as_secs_f64());
            let started = Instant::now();
            let result = self
                .http
                .get(&url)
                .query(query)
                .header(API_KEY_HEADER, self.api_key.clone())
                .header(ACCEPT, "application/json")
                .send()
                .await;
            let status = match &result {
                Ok(response) => response.status().as_u16().to_string(),
                Err(e) => transport_label(e).to_owned(),
            };
            metrics::histogram!("umamoe_request_duration_seconds", "endpoint" => path.to_owned(), "lane" => lane)
                .record(started.elapsed().as_secs_f64());
            metrics::counter!("umamoe_requests_total", "endpoint" => path.to_owned(), "lane" => lane, "status" => status.clone())
                .increment(1);

            let (error, retry_after) = match result {
                Ok(response) if response.status().is_success() => {
                    let body = response.bytes().await.map_err(Error::Transport)?;
                    return serde_json::from_slice(&body).map_err(|e| {
                        metrics::counter!("umamoe_decode_errors_total", "endpoint" => path.to_owned())
                            .increment(1);
                        Error::Decode(e)
                    });
                }
                Ok(response) => {
                    let status = response.status();
                    let retry_after = retry::retry_after(response.headers());
                    let body = response
                        .bytes()
                        .await
                        .ok()
                        .and_then(|b| serde_json::from_slice::<ApiErrorBody>(&b).ok());
                    let error = Error::Status { status, body };
                    if !retry::is_retryable_status(status) {
                        return Err(error);
                    }
                    (error, retry_after)
                }
                Err(e) if retry::is_retryable_error(&e) => (Error::Transport(e), None),
                Err(e) => return Err(Error::Transport(e)),
            };

            if retries >= self.retry.max_retries {
                return Err(error);
            }
            metrics::counter!("umamoe_retries_total", "endpoint" => path.to_owned(), "status" => status)
                .increment(1);
            let delay = retry::delay(&self.retry, retries, retry_after, fastrand::f64());
            tokio::time::sleep(delay).await;
            retries += 1;
        }
    }
}

fn lane_label(priority: Priority) -> &'static str {
    match priority {
        Priority::Interactive => "interactive",
        Priority::Background => "background",
    }
}

fn transport_label(error: &reqwest::Error) -> &'static str {
    if error.is_timeout() {
        "timeout"
    } else if error.is_connect() {
        "connect_error"
    } else {
        "transport_error"
    }
}
