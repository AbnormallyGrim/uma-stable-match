//! The Discord application: its settings and clients.

use std::sync::Arc;

use crate::config::DiscordConfig;
use crate::error::Error;
use crate::names::Names;
use crate::opt_in::OptIn;
use crate::rest::Rest;
use crate::trainers::TrainerLinks;

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
    opt_in: Option<OptIn>,
    trainer_links: Option<TrainerLinks>,
    names: Arc<Names>,
}

impl Discord {
    /// The application with these settings.
    pub fn new(config: DiscordConfig) -> Result<Self, Error> {
        let rest = Rest::new(&config)?;
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
        Ok(Self {
            inner: Arc::new(Inner {
                config,
                rest,
                http,
                opt_in: None,
                trainer_links: None,
                names: Arc::default(),
            }),
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

    /// The HTTP client for calls made as the signed-in user.
    pub(crate) fn http(&self) -> &reqwest::Client {
        &self.inner.http
    }

    /// The same application, also serving the opt-in routes with `opt_in`.
    pub fn with_opt_in(self, opt_in: OptIn) -> Self {
        let inner = &self.inner;
        Self {
            inner: Arc::new(Inner {
                config: inner.config.clone(),
                rest: inner.rest.clone(),
                http: inner.http.clone(),
                opt_in: Some(opt_in),
                trainer_links: inner.trainer_links.clone(),
                names: inner.names.clone(),
            }),
        }
    }

    /// The same application, also serving the trainer-link routes with `trainer_links`.
    pub fn with_trainer_links(self, trainer_links: TrainerLinks) -> Self {
        let inner = &self.inner;
        Self {
            inner: Arc::new(Inner {
                config: inner.config.clone(),
                rest: inner.rest.clone(),
                http: inner.http.clone(),
                opt_in: inner.opt_in.clone(),
                trainer_links: Some(trainer_links),
                names: inner.names.clone(),
            }),
        }
    }

    pub(crate) fn opt_in(&self) -> Option<&OptIn> {
        self.inner.opt_in.as_ref()
    }

    pub(crate) fn trainer_links(&self) -> Option<&TrainerLinks> {
        self.inner.trainer_links.as_ref()
    }

    pub(crate) fn names(&self) -> &Names {
        &self.inner.names
    }
}
