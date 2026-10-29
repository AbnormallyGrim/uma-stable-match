//! Starts the Uma Stable Match server with uma.moe as its data source.

use std::process::ExitCode;

use uma_stable_match_server::{init_logging, run, Config, LogFormat, TraceExport, UmaMoeConfig};
use uma_stable_match_umamoe_client::Client;

#[tokio::main]
async fn main() -> ExitCode {
    dotenvy::dotenv().ok();
    let log_format = LogFormat::from_env();
    let traces = TraceExport::from_env();
    init_logging(
        log_format.clone().unwrap_or_default(),
        traces.as_ref().ok().and_then(Option::as_ref),
    );
    if let Err(error) = log_format {
        tracing::error!("{error}");
        return ExitCode::FAILURE;
    }
    let traces = match traces {
        Ok(traces) => traces,
        Err(error) => {
            tracing::error!("{error}");
            return ExitCode::FAILURE;
        }
    };
    if traces.is_some() {
        tracing::info!("exporting traces over OTLP");
    }

    let config = match Config::from_env() {
        Ok(config) => config,
        Err(error) => {
            tracing::error!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let client = match UmaMoeConfig::from_env()
        .map_err(|error| error.to_string())
        .and_then(|umamoe| Client::new(umamoe.client_config()).map_err(|error| error.to_string()))
    {
        Ok(client) => client,
        Err(error) => {
            tracing::error!("{error}");
            return ExitCode::FAILURE;
        }
    };

    let result = run(&config, client).await;
    if let Some(traces) = traces {
        traces.shutdown();
    }
    if let Err(error) = result {
        tracing::error!("{error}");
        return ExitCode::FAILURE;
    }
    tracing::info!("stopped");
    ExitCode::SUCCESS
}
