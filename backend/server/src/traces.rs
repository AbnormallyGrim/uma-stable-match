//! Optional trace export over OTLP.

use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::{ExporterBuildError, SpanExporter};
use opentelemetry_sdk::trace::{SdkTracer, SdkTracerProvider};
use opentelemetry_sdk::Resource;

const SERVICE_NAME: &str = "uma-stable-match";
const TRACER_NAME: &str = "uma-stable-match-server";

/// Sends spans to an OTLP endpoint. Keep it until shutdown, then call [`TraceExport::shutdown`].
#[derive(Debug, Clone)]
pub struct TraceExport {
    provider: SdkTracerProvider,
}

/// The OTLP exporter couldn't be built from the `OTEL_*` settings.
#[derive(Debug, thiserror::Error)]
#[error("could not start trace export: {0}")]
pub struct TraceExportError(#[from] ExporterBuildError);

impl TraceExport {
    /// Starts trace export if `OTEL_EXPORTER_OTLP_ENDPOINT` or `OTEL_EXPORTER_OTLP_TRACES_ENDPOINT`
    /// is set and `OTEL_SDK_DISABLED` isn't `true`. The other `OTEL_*` settings are read too.
    pub fn from_env() -> Result<Option<Self>, TraceExportError> {
        let lookup = |name: &str| std::env::var(name).ok();
        if !is_enabled(lookup) {
            return Ok(None);
        }
        let exporter = SpanExporter::builder().with_http().build()?;
        Ok(Some(Self::with_exporter(exporter, lookup)))
    }

    fn with_exporter(
        exporter: impl opentelemetry_sdk::trace::SpanExporter + 'static,
        lookup: impl Fn(&str) -> Option<String>,
    ) -> Self {
        let mut resource = Resource::builder();
        if blank(lookup("OTEL_SERVICE_NAME")) {
            resource = resource.with_service_name(SERVICE_NAME);
        }
        let provider = SdkTracerProvider::builder()
            .with_batch_exporter(exporter)
            .with_resource(resource.build())
            .build();
        Self { provider }
    }

    /// The tracer that [`init_logging`](crate::init_logging) sends spans to.
    pub fn tracer(&self) -> SdkTracer {
        self.provider.tracer(TRACER_NAME)
    }

    /// Sends the spans still waiting and stops exporting.
    pub fn shutdown(self) {
        if let Err(error) = self.provider.shutdown() {
            tracing::warn!("could not flush traces: {error}");
        }
    }
}

fn is_enabled(lookup: impl Fn(&str) -> Option<String>) -> bool {
    let disabled =
        lookup("OTEL_SDK_DISABLED").is_some_and(|value| value.trim().eq_ignore_ascii_case("true"));
    let endpoint = !blank(lookup("OTEL_EXPORTER_OTLP_ENDPOINT"))
        || !blank(lookup("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT"));
    endpoint && !disabled
}

fn blank(value: Option<String>) -> bool {
    value.is_none_or(|value| value.trim().is_empty())
}

#[cfg(test)]
mod tests;
