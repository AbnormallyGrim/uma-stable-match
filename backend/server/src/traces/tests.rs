use super::*;
use std::collections::HashMap;

use opentelemetry_sdk::trace::{InMemorySpanExporter, SpanData};
use tracing_subscriber::EnvFilter;

use axum::body::Body;
use axum::extract::Request;
use axum::middleware::from_fn;
use axum::routing::get;
use axum::Router;
use tower::ServiceExt;

use crate::logging::{subscriber, LogFormat};
use crate::telemetry::track_requests;

fn vars(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let map: HashMap<String, String> = pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect();
    move |name| map.get(name).cloned()
}

#[test]
fn needs_an_endpoint() {
    assert!(!is_enabled(vars(&[])));
    assert!(!is_enabled(vars(&[("OTEL_EXPORTER_OTLP_ENDPOINT", " ")])));
    assert!(is_enabled(vars(&[(
        "OTEL_EXPORTER_OTLP_ENDPOINT",
        "http://127.0.0.1:4318"
    )])));
    assert!(is_enabled(vars(&[(
        "OTEL_EXPORTER_OTLP_TRACES_ENDPOINT",
        "http://127.0.0.1:4318/v1/traces"
    )])));
}

#[test]
fn can_be_disabled() {
    assert!(!is_enabled(vars(&[
        ("OTEL_EXPORTER_OTLP_ENDPOINT", "http://127.0.0.1:4318"),
        ("OTEL_SDK_DISABLED", "TRUE"),
    ])));
    assert!(is_enabled(vars(&[
        ("OTEL_EXPORTER_OTLP_ENDPOINT", "http://127.0.0.1:4318"),
        ("OTEL_SDK_DISABLED", "false"),
    ])));
}

// Reads the spans before the provider drops, since shutting down clears the in-memory exporter.
fn export(run: impl FnOnce()) -> Vec<SpanData> {
    let exporter = InMemorySpanExporter::default();
    let traces = TraceExport::with_exporter(exporter.clone(), vars(&[]));
    let subscriber = subscriber(
        LogFormat::Text,
        EnvFilter::new("info"),
        std::io::sink,
        Some(traces.tracer()),
    );
    tracing::subscriber::with_default(subscriber, run);
    traces.provider.force_flush().unwrap();
    exporter.get_finished_spans().unwrap()
}

#[test]
fn exports_spans_without_log_lines() {
    let spans = export(|| {
        let span = tracing::info_span!("work", circle_id = 1_u64);
        let _entered = span.enter();
        tracing::info!(trainer_id = 123_456_789_u64, "search");
    });
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].name, "work");
    assert!(spans[0].events.is_empty());
    assert!(spans[0]
        .attributes
        .iter()
        .all(|kv| kv.key.as_str() != "trainer_id"));
}

#[test]
fn requests_get_a_span_without_their_query() {
    let app = Router::new()
        .route("/api/v1/thing", get(|| async { "ok" }))
        .layer(from_fn(track_requests));
    let spans = export(|| {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let request = Request::get("/api/v1/thing?trainer_id=123456789")
            .body(Body::empty())
            .unwrap();
        runtime.block_on(app.oneshot(request)).unwrap();
    });
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].name, "GET /api/v1/thing");
    let attribute = |key: &str| {
        spans[0]
            .attributes
            .iter()
            .find(|kv| kv.key.as_str() == key)
            .map(|kv| kv.value.to_string())
    };
    assert_eq!(attribute("http.route").as_deref(), Some("/api/v1/thing"));
    assert_eq!(
        attribute("http.response.status_code").as_deref(),
        Some("200")
    );
    assert!(spans[0]
        .attributes
        .iter()
        .all(|kv| !kv.value.to_string().contains("123456789")));
}
