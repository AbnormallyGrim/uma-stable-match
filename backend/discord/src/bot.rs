//! The Discord application: its settings and REST client.

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
}

impl Discord {
    /// The application with these settings.
    pub fn new(config: DiscordConfig) -> Result<Self, Error> {
        let rest = Rest::new(&config)?;
        Ok(Self {
            inner: Arc::new(Inner { config, rest }),
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
}
