//! Error, log and trace reporting to Sentry.

use sentry::ClientInitGuard;

/// Starts Sentry if `SENTRY_DSN` is set, and returns the guard that sends what is queued when it
/// drops. Call it before [`init_logging`](crate::init_logging) and keep the guard until shutdown.
///
/// The SDK reads `SENTRY_DSN`, `SENTRY_ENVIRONMENT` and `SENTRY_RELEASE` itself.
/// `SENTRY_TRACES_SAMPLE_RATE` sets the share of requests traced (default `0.1`). No personal data
/// is sent.
pub fn init_sentry() -> ClientInitGuard {
    let rate = std::env::var("SENTRY_TRACES_SAMPLE_RATE")
        .ok()
        .and_then(|rate| rate.trim().parse::<f32>().ok())
        .filter(|rate| (0.0..=1.0).contains(rate))
        .unwrap_or(0.1);
    sentry::init(
        sentry::ClientOptions::new()
            .traces_sample_rate(rate)
            .send_default_pii(false),
    )
}
