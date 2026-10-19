use std::process::ExitCode;

use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;
use uma_stable_match_server::{router, Config};

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

    let listener = match TcpListener::bind(config.bind_addr).await {
        Ok(listener) => listener,
        Err(error) => {
            tracing::error!("could not listen on {}: {error}", config.bind_addr);
            return ExitCode::FAILURE;
        }
    };
    tracing::info!("listening on {}", config.bind_addr);

    if let Err(error) = axum::serve(listener, router())
        .with_graceful_shutdown(shutdown_signal())
        .await
    {
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
