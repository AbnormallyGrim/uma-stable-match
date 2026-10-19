//! Starts the Uma Stable Match server.

use std::process::ExitCode;

use tokio::net::TcpListener;
use tokio::sync::watch;
use tracing_subscriber::EnvFilter;
use uma_stable_match_server::{
    router, run_scheduler, AppState, Cache, Config, PaceLookup, DEFAULT_PACE_TTL,
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
    let paces = PaceLookup::new(client.clone(), DEFAULT_PACE_TTL);
    let scheduler = tokio::spawn(run_scheduler(
        client,
        config.scan,
        config.scan_interval,
        cache.clone(),
        stopped,
    ));

    let served = axum::serve(listener, router(AppState { cache, paces }))
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
