//! Starts the Uma Stable Match server.

use std::net::SocketAddr;
use std::process::ExitCode;

use tokio::net::TcpListener;
use tokio::sync::watch;
use tracing_subscriber::EnvFilter;
use uma_stable_match_server::{
    app, install_metrics, metrics_router, run_scheduler, AppState, Cache, ClubDetails, Config,
    DetailConfig, PaceLookup, RateLimits, RefreshConfig, SharedCache, SharedPool,
};
use uma_stable_match_umamoe_client::{Client, ClientConfig};

#[tokio::main]
async fn main() -> ExitCode {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = match Config::from_env() {
        Ok(config) => config,
        Err(error) => {
            tracing::error!("{error}");
            return ExitCode::FAILURE;
        }
    };

    let metrics = match config.metrics_addr {
        None => None,
        Some(addr) => {
            let handle = match install_metrics() {
                Ok(handle) => handle,
                Err(error) => {
                    tracing::error!("could not start metrics: {error}");
                    return ExitCode::FAILURE;
                }
            };
            let listener = match TcpListener::bind(addr).await {
                Ok(listener) => listener,
                Err(error) => {
                    tracing::error!("could not serve metrics on {addr}: {error}");
                    return ExitCode::FAILURE;
                }
            };
            tracing::info!("serving metrics on {addr}");
            Some((handle, listener))
        }
    };

    let mut client_config = ClientConfig::new(config.api_key.clone());
    if let Some(base_url) = &config.base_url {
        client_config.base_url = base_url.clone();
    }
    let client = match Client::new(client_config) {
        Ok(client) => client,
        Err(error) => {
            tracing::error!("{error}");
            return ExitCode::FAILURE;
        }
    };

    let shared = match (&config.redis_url, &config.redis_key_secret) {
        (Some(url), Some(secret)) => match SharedCache::connect(url, secret.clone()).await {
            Ok(shared) => {
                tracing::info!("using Redis as a shared cache");
                shared
            }
            Err(error) => {
                tracing::error!("{error}");
                return ExitCode::FAILURE;
            }
        },
        _ => SharedCache::none(),
    };

    let listener = match TcpListener::bind(config.bind_addr).await {
        Ok(listener) => listener,
        Err(error) => {
            tracing::error!("could not listen on {}: {error}", config.bind_addr);
            return ExitCode::FAILURE;
        }
    };
    tracing::info!("listening on {}", config.bind_addr);

    let cache = Cache::new();
    let (stop, stopped) = watch::channel(false);
    let paces = PaceLookup::new(client.clone(), config.trainer_cache).with_shared(shared.clone());
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
        &client,
        SharedPool::default(),
        DetailConfig {
            max_clubs: window * 2,
            queue_size: window as usize,
            refresh,
            refresher: !shared.is_enabled(),
        },
        shared,
    );
    tokio::spawn(loader.run());
    if let Some(refresher) = details.refresher() {
        tokio::spawn(refresher.run());
    }
    let scheduler = tokio::spawn(run_scheduler(
        client,
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
        paces,
        limits,
        details,
        matching: config.matching,
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
    let app = app(state, config.frontend_dir.as_deref());
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

    if let Err(error) = served {
        tracing::error!("server error: {error}");
        return ExitCode::FAILURE;
    }
    tracing::info!("stopped");
    ExitCode::SUCCESS
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
