//! The API client.

use std::fmt;

use reqwest::header::{HeaderValue, ACCEPT};
use reqwest::Url;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::config::ClientConfig;
use crate::error::{ApiErrorBody, Error};
use crate::requests::{CircleRequest, ListCirclesRequest};
use crate::types::{CircleListResponse, CircleResponse};

const API_KEY_HEADER: &str = "X-API-Key";

/// uma.moe API client. Clones share one connection pool.
#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    base_url: String,
    api_key: HeaderValue,
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client")
            .field("base_url", &self.base_url)
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
        })
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

    pub async fn list_circles(
        &self,
        request: &ListCirclesRequest,
    ) -> Result<CircleListResponse, Error> {
        self.get("/api/v4/circles/list", request).await
    }

    async fn get<T, Q>(&self, path: &str, query: &Q) -> Result<T, Error>
    where
        T: DeserializeOwned,
        Q: Serialize + ?Sized,
    {
        let response = self
            .http
            .get(format!("{}{path}", self.base_url))
            .query(query)
            .header(API_KEY_HEADER, self.api_key.clone())
            .header(ACCEPT, "application/json")
            .send()
            .await
            .map_err(Error::Transport)?;

        let status = response.status();
        let body = response.bytes().await.map_err(Error::Transport)?;
        if !status.is_success() {
            let body = serde_json::from_slice::<ApiErrorBody>(&body).ok();
            return Err(Error::Status { status, body });
        }
        serde_json::from_slice(&body).map_err(Error::Decode)
    }
}
