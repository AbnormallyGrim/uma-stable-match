//! Error, log and trace reporting to Sentry.

use sentry::protocol::Log;
use sentry::ClientInitGuard;

/// The log attribute and issue tag naming the Fly app that sent them.
const APP_KEY: &str = "fly.app";

/// Starts Sentry if `SENTRY_DSN` is set, and returns the guard that sends what is queued when it
/// drops. Call it before [`init_logging`](crate::init_logging) and keep the guard until shutdown.
///
/// The SDK reads `SENTRY_DSN`, `SENTRY_ENVIRONMENT` and `SENTRY_RELEASE` itself.
/// `SENTRY_TRACES_SAMPLE_RATE` sets the share of requests traced (default `0.1`). On Fly.io, logs
/// and issues are marked with the app's name (`fly.app`). No personal data is sent.
pub fn init_sentry() -> ClientInitGuard {
    let rate = std::env::var("SENTRY_TRACES_SAMPLE_RATE")
        .ok()
        .and_then(|rate| rate.trim().parse::<f32>().ok())
        .filter(|rate| (0.0..=1.0).contains(rate))
        .unwrap_or(0.1);
    let app = std::env::var("FLY_APP_NAME")
        .ok()
        .map(|app| app.trim().to_owned())
        .filter(|app| !app.is_empty());
    let mut options = sentry::ClientOptions::new()
        .traces_sample_rate(rate)
        .send_default_pii(false);
    if let Some(app) = app.clone() {
        options = options.before_send_log(move |log| Some(mark_app(log, &app)));
    }
    let guard = sentry::init(options);
    if let Some(app) = app {
        sentry::configure_scope(|scope| scope.set_tag(APP_KEY, app));
    }
    guard
}

fn mark_app(mut log: Log, app: &str) -> Log {
    log.attributes
        .insert(APP_KEY.to_owned(), app.to_owned().into());
    log
}

#[cfg(test)]
mod tests {
    use super::*;
    use sentry::protocol::{LogAttribute, LogLevel, Value};
    use std::time::SystemTime;

    #[test]
    fn marks_logs_with_the_app() {
        let log = Log {
            level: LogLevel::Warn,
            body: "example".to_owned(),
            trace_id: None,
            timestamp: SystemTime::now(),
            severity_number: None,
            attributes: Default::default(),
        };
        let log = mark_app(log, "example-app");
        assert_eq!(
            log.attributes.get(APP_KEY),
            Some(&LogAttribute(Value::from("example-app")))
        );
    }
}
