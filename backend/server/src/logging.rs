use tracing::Subscriber;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogFormat {
    #[default]
    Text,
    Json,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("LOG_FORMAT must be text or json, got {0:?}")]
pub struct InvalidLogFormat(pub String);

impl LogFormat {
    pub fn from_env() -> Result<Self, InvalidLogFormat> {
        Self::parse(std::env::var("LOG_FORMAT").ok().as_deref())
    }

    pub fn parse(value: Option<&str>) -> Result<Self, InvalidLogFormat> {
        match value.map(str::trim) {
            None | Some("" | "text") => Ok(Self::Text),
            Some("json") => Ok(Self::Json),
            Some(other) => Err(InvalidLogFormat(other.to_owned())),
        }
    }
}

pub fn init_logging(format: LogFormat) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    subscriber(format, filter, std::io::stdout).init();
}

fn subscriber<W>(
    format: LogFormat,
    filter: EnvFilter,
    writer: W,
) -> Box<dyn Subscriber + Send + Sync>
where
    W: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let builder = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(writer);
    match format {
        LogFormat::Text => Box::new(builder.finish()),
        LogFormat::Json => Box::new(
            builder
                .json()
                .flatten_event(true)
                .with_current_span(false)
                .with_span_list(false)
                .finish(),
        ),
    }
}

#[cfg(test)]
mod tests;
