//! The Discord application: its settings and clients.

use std::sync::Arc;

use crate::config::DiscordConfig;
use crate::error::Error;
use crate::rest::Rest;

/// The Discord application, shared by every route. Cheap to clone.
#[derive(Debug, Clone)]
pub struct Discord {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    config: DiscordConfig,
    rest: Rest,
    http: reqwest::Client,
}

impl Discord {
    /// The application with these settings.
    pub fn new(config: DiscordConfig) -> Result<Self, Error> {
        let rest = Rest::new(&config)?;
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
        Ok(Self {
            inner: Arc::new(Inner { config, rest, http }),
        })
    }

    /// The application's settings.
    pub fn config(&self) -> &DiscordConfig {
        &self.inner.config
    }

    /// The REST client, authenticated as the bot.
    pub fn rest(&self) -> &Rest {
        &self.inner.rest
    }

    /// The HTTP client for calls made as the signed-in user instead of the bot.
    pub(crate) fn http(&self) -> &reqwest::Client {
        &self.inner.http
    }
}
