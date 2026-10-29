//! Starts the server with a data source and runs it until shutdown.

use std::io;
use std::net::SocketAddr;

use metrics_exporter_prometheus::BuildError;
use tokio::net::TcpListener;
use tokio::sync::watch;

use crate::app::{app_with_features, AppState};
use crate::cache::Cache;
use crate::config::Config;
use crate::details::{ClubDetails, DetailConfig, SharedPool};
use crate::limits::RateLimits;
use crate::pace::PaceLookup;
use crate::refresher::RefreshConfig;
use crate::scheduler::run_scheduler;
use crate::shared::{SharedCache, SharedCacheError};
use crate::source::Source;
use crate::telemetry::{install_metrics, metrics_router};

/// Why the server couldn't start or stopped with an error.
#[derive(Debug, thiserror::Error)]
pub enum RunError {
    /// The metrics recorder couldn't be installed.
    #[error("could not start metrics: {0}")]
    Metrics(#[source] BuildError),
    /// The metrics address couldn't be bound.
    #[error("could not serve metrics on {addr}: {source}")]
    MetricsAddr {
        /// `METRICS_ADDR`.
        addr: SocketAddr,
        /// The bind error.
        source: io::Error,
    },
    /// Redis couldn't be reached.
    #[error(transparent)]
    Redis(#[from] SharedCacheError),
    /// The listen address couldn't be bound.
    #[error("could not listen on {addr}: {source}")]
    Listen {
        /// `HOST` and `PORT`.
        addr: SocketAddr,
        /// The bind error.
        source: io::Error,
    },
    /// Serving failed.
    #[error("server error: {0}")]
    Serve(#[source] io::Error),
}

/// Serves the API, and the frontend if configured, with club data from `source`, until Ctrl-C or
/// SIGTERM. Logging and trace export are the caller's to set up and shut down.
pub async fn run(config: &Config, source: impl Into<Source>) -> Result<(), RunError> {
    let source = source.into();
    let metrics = match config.metrics_addr {
        None => None,
        Some(addr) => {
            let handle = install_metrics().map_err(RunError::Metrics)?;
            let listener = TcpListener::bind(addr)
                .await
                .map_err(|source| RunError::MetricsAddr { addr, source })?;
            tracing::info!("serving metrics on {addr}");
            Some((handle, listener))
        }
    };

    let shared = match (&config.redis_url, &config.redis_key_secret) {
        (Some(url), Some(secret)) => {
            let shared = SharedCache::connect(url, secret.clone()).await?;
            tracing::info!("using Redis as a shared cache");
            shared
        }
        _ => SharedCache::none(),
    };

    let listener = TcpListener::bind(config.bind_addr)
        .await
        .map_err(|source| RunError::Listen {
            addr: config.bind_addr,
            source,
        })?;
    tracing::info!("listening on {}", config.bind_addr);

    let cache = Cache::new();
    let (stop, stopped) = watch::channel(false);
    let paces = PaceLookup::new(&source, config.trainer_cache).with_shared(shared.clone());
    let limits = RateLimits::new(
        config.rate_limit_per_minute,
        config.trainer_lookups_per_minute,
        config.trust_forwarded_for,
    );
    let window = u64::from(config.scan.rank_to - config.scan.rank_from + 1);
    let refresh = RefreshConfig {
        every: config.detail_cache,
        ..RefreshConfig::default()
    };
    let (details, loader) = ClubDetails::with_shared(
        &source,
        SharedPool::default(),
        DetailConfig {
            max_bytes: u64::from(config.detail_cache_mb) * 1024 * 1024,
            queue_size: window as usize,
            refresh,
            refresher: true,
            scan: config.scan,
        },
        shared,
    );
    tokio::spawn(loader.run());
    if let Some(refresher) = details.refresher() {
        tokio::spawn(refresher.run());
    }
    let scheduler = tokio::spawn(run_scheduler(
        source,
        details.clone(),
        config.scan,
        config.scan_interval,
        cache.clone(),
        stopped,
    ));

    if let Some(dir) = &config.frontend_dir {
        tracing::info!("serving the frontend from {}", dir.display());
    }
    let state = AppState {
        cache,
        paces: paces.with_details(details.clone()),
        limits,
        details,
        matching: config.matching,
        human: config.human_check(),
    };
    if let Some((handle, listener)) = metrics {
        let mut stopped = stop.subscribe();
        let router = metrics_router(handle, state.clone());
        tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    stopped.wait_for(|stop| *stop).await.ok();
                })
                .await
                .ok();
        });
    }
    let app = app_with_features(state, config.frontend_dir.as_deref(), config.features);
    let served = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        shutdown_signal().await;
        stop.send(true).ok();
    })
    .await;
    scheduler.await.ok();
    served.map_err(RunError::Serve)
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.ok();
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(_) => std::future::pending().await,
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutting down");
}
