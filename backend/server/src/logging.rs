//! Log output, as readable text or JSON lines.

use std::backtrace::Backtrace;

use opentelemetry_sdk::trace::SdkTracer;
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::filter::filter_fn;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use crate::traces::TraceExport;

/// How log lines are written, from `LOG_FORMAT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogFormat {
    /// Human-readable lines.
    #[default]
    Text,
    /// One JSON object per line, with the event's fields at the top level.
    Json,
}

/// A `LOG_FORMAT` other than `text` or `json`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("LOG_FORMAT must be text or json, got {0:?}")]
pub struct InvalidLogFormat(pub String);

impl LogFormat {
    /// Reads `LOG_FORMAT`. Unset or blank means [`LogFormat::Text`].
    pub fn from_env() -> Result<Self, InvalidLogFormat> {
        Self::parse(std::env::var("LOG_FORMAT").ok().as_deref())
    }

    /// Parses a `LOG_FORMAT` value. `None` or blank means [`LogFormat::Text`].
    pub fn parse(value: Option<&str>) -> Result<Self, InvalidLogFormat> {
        match value.map(str::trim) {
            None | Some("" | "text") => Ok(Self::Text),
            Some("json") => Ok(Self::Json),
            Some(other) => Err(InvalidLogFormat(other.to_owned())),
        }
    }
}

/// Sends logs to stdout in `format`, filtered by `RUST_LOG` (default `info`), and logs panics with a
/// stack trace. With `traces`, spans are exported too, but log lines aren't. Call it once.
pub fn init_logging(format: LogFormat, traces: Option<&TraceExport>) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    subscriber(
        format,
        filter,
        std::io::stdout,
        traces.map(TraceExport::tracer),
    )
    .init();
    std::panic::set_hook(Box::new(log_panic));
}

/// Logs a panic as one error line with its message, location and a stack trace.
fn log_panic(info: &std::panic::PanicHookInfo<'_>) {
    let payload = info.payload();
    let message = payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("panic");
    tracing::error!(
        origin = "server",
        panic = message,
        location = info.location().map(|l| format!("{}:{}", l.file(), l.line())),
        backtrace = %Backtrace::force_capture(),
        "panicked"
    );
}

pub(crate) fn subscriber<W>(
    format: LogFormat,
    filter: EnvFilter,
    writer: W,
    tracer: Option<SdkTracer>,
) -> Box<dyn Subscriber + Send + Sync>
where
    W: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let lines = tracing_subscriber::fmt::layer()
        .with_writer(writer)
        .with_file(true)
        .with_line_number(true);
    let lines = match format {
        LogFormat::Text => lines.boxed(),
        LogFormat::Json => lines
            .json()
            .flatten_event(true)
            .with_current_span(false)
            .with_span_list(false)
            .boxed(),
    };
    Box::new(
        tracing_subscriber::registry()
            .with(filter)
            .with(lines)
            .with(CountProblems)
            .with(export_spans(tracer)),
    )
}

// Events stay out of exported spans, so traces never hold what log lines do.
fn export_spans<S>(tracer: Option<SdkTracer>) -> Option<impl Layer<S>>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    tracer.map(|tracer| {
        tracing_opentelemetry::layer()
            .with_tracer(tracer)
            .with_filter(filter_fn(|metadata| metadata.is_span()))
    })
}

/// Counts every warning and error as `log_events_total`, by level and the file and line that logged
/// it. Other crates' lines are counted without a file or line, so the label set stays small.
struct CountProblems;

impl<S: Subscriber> Layer<S> for CountProblems {
    fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
        let metadata = event.metadata();
        let level = match *metadata.level() {
            Level::ERROR => "error",
            Level::WARN => "warn",
            _ => return,
        };
        let (file, line) = if metadata.target().starts_with("uma_stable_match") {
            (
                metadata.file().unwrap_or_default(),
                metadata
                    .line()
                    .map(|line| line.to_string())
                    .unwrap_or_default(),
            )
        } else {
            ("", String::new())
        };
        metrics::counter!("log_events_total", "level" => level, "file" => file, "line" => line)
            .increment(1);
    }
}

#[cfg(test)]
mod tests;
