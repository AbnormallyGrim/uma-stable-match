use super::*;
use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);

impl io::Write for Captured {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for Captured {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

fn capture(format: LogFormat, log: impl FnOnce()) -> String {
    let captured = Captured::default();
    let subscriber = subscriber(format, EnvFilter::new("info"), captured.clone(), None);
    tracing::subscriber::with_default(subscriber, log);
    let bytes = captured.0.lock().unwrap().clone();
    String::from_utf8(bytes).unwrap()
}

#[test]
fn parses_formats() {
    assert_eq!(LogFormat::parse(None), Ok(LogFormat::Text));
    assert_eq!(LogFormat::parse(Some("")), Ok(LogFormat::Text));
    assert_eq!(LogFormat::parse(Some("text")), Ok(LogFormat::Text));
    assert_eq!(LogFormat::parse(Some(" json ")), Ok(LogFormat::Json));
}

#[test]
fn rejects_unknown_formats() {
    let error = LogFormat::parse(Some("yaml")).unwrap_err();
    assert_eq!(error, InvalidLogFormat("yaml".to_owned()));
    assert_eq!(
        error.to_string(),
        r#"LOG_FORMAT must be text or json, got "yaml""#
    );
}

#[test]
fn json_writes_one_object_per_line_with_flat_fields() {
    let output = capture(LogFormat::Json, || {
        let error = "timed out";
        tracing::warn!(circle_id = 900_000_101_u64, %error, "club refresh failed");
        tracing::info!("scan finished");
        tracing::debug!("filtered out");
    });
    let lines: Vec<serde_json::Value> = output
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["level"], "WARN");
    assert_eq!(lines[0]["message"], "club refresh failed");
    assert_eq!(lines[0]["circle_id"], 900_000_101_u64);
    assert_eq!(lines[0]["error"], "timed out");
    assert!(lines[0]["timestamp"].is_string());
    assert_eq!(lines[1]["message"], "scan finished");
}

#[test]
fn text_keeps_readable_lines() {
    let output = capture(LogFormat::Text, || {
        tracing::info!(circle_id = 7, "scan finished")
    });
    assert!(output.contains("INFO"));
    assert!(output.contains("scan finished"));
    assert!(serde_json::from_str::<serde_json::Value>(output.trim()).is_err());
}

fn json_lines(output: &str) -> Vec<serde_json::Value> {
    output
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn json_lines_say_where_they_were_logged() {
    let output = capture(LogFormat::Json, || tracing::warn!("club refresh failed"));
    let line = &json_lines(&output)[0];
    assert!(line["filename"]
        .as_str()
        .unwrap()
        .ends_with("logging/tests.rs"));
    assert!(line["line_number"].as_u64().unwrap() > 0);
}

#[test]
fn panics_are_logged_with_their_location_and_a_backtrace() {
    let output = capture(LogFormat::Json, || {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(log_panic));
        let _ = std::panic::catch_unwind(|| panic!("made-up failure"));
        std::panic::set_hook(previous);
    });
    let line = &json_lines(&output)[0];
    assert_eq!(line["level"], "ERROR");
    assert_eq!(line["message"], "panicked");
    assert_eq!(line["origin"], "server");
    assert_eq!(line["panic"], "made-up failure");
    assert!(line["location"]
        .as_str()
        .unwrap()
        .contains("logging/tests.rs"));
    assert!(!line["backtrace"].as_str().unwrap().is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn failed_umamoe_requests_are_logged_once_with_their_origin() {
    use uma_stable_match_umamoe_client::{
        ApiKey, CircleRequest, Client, ClientConfig, RetryConfig,
    };
    use wiremock::matchers::{path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", "1"))
        .respond_with(
            ResponseTemplate::new(500).set_body_json(
                serde_json::json!({ "error": "database unavailable", "status": 500 }),
            ),
        )
        .mount(&server)
        .await;
    Mock::given(path("/api/v4/circles"))
        .and(query_param("circle_id", "2"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    let client = Client::new(ClientConfig {
        base_url: server.uri(),
        min_interval: Duration::ZERO,
        retry: RetryConfig {
            max_retries: 1,
            base_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(5),
        },
        ..ClientConfig::new(ApiKey::new("test-key"))
    })
    .unwrap();

    let captured = Captured::default();
    let _guard = tracing::subscriber::set_default(subscriber(
        LogFormat::Json,
        EnvFilter::new("info"),
        captured.clone(),
        None,
    ));
    client
        .circle(&CircleRequest::by_circle_id(1))
        .await
        .unwrap_err();
    client
        .circle(&CircleRequest::by_circle_id(2))
        .await
        .unwrap_err();
    drop(_guard);

    let output = String::from_utf8(captured.0.lock().unwrap().clone()).unwrap();
    let failures: Vec<_> = json_lines(&output)
        .into_iter()
        .filter(|line| line["message"] == "uma.moe request failed")
        .collect();
    assert_eq!(failures.len(), 1);
    let line = &failures[0];
    assert_eq!(line["level"], "WARN");
    assert_eq!(line["endpoint"], "/api/v4/circles");
    assert_eq!(line["query"], "circle_id=1");
    assert_eq!(line["origin"], "umamoe");
    assert_eq!(line["status"], 500);
    assert_eq!(line["umamoe_error"], "database unavailable");
    assert_eq!(line["attempts"], 2);
    assert!(line["backtrace"].is_string());
}

#[test]
fn warnings_and_errors_are_counted_by_where_they_were_logged() {
    let recorder = metrics_exporter_prometheus::PrometheusBuilder::new().build_recorder();
    let handle = recorder.handle();
    metrics::with_local_recorder(&recorder, || {
        let captured = Captured::default();
        let subscriber = subscriber(LogFormat::Json, EnvFilter::new("info"), captured, None);
        tracing::subscriber::with_default(subscriber, || {
            tracing::warn!("club refresh failed");
            tracing::warn!(target: "hyper", "connection reset");
            tracing::error!("scan failed");
            tracing::info!("scan finished");
        });
    });
    let rendered = handle.render();
    let counted = |level: &str, file: &str| {
        rendered
            .lines()
            .filter(|line| line.starts_with("log_events_total{"))
            .filter(|line| line.contains(&format!(r#"level="{level}",file="{file}",line=""#)))
            .filter_map(|line| {
                let number = line.split(r#"line=""#).nth(1)?.split('"').next()?;
                (file.is_empty() == number.is_empty()).then_some(line.ends_with(" 1"))
            })
            .filter(|&one| one)
            .count()
    };
    let here = "backend/server/src/logging/tests.rs";
    assert_eq!(counted("warn", here), 1);
    assert_eq!(counted("error", here), 1);
    assert_eq!(counted("warn", ""), 1);
    assert!(!rendered.contains(r#"level="info""#));
}
