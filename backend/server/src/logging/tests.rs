use super::*;
use std::io;
use std::sync::{Arc, Mutex};

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
    let subscriber = subscriber(format, EnvFilter::new("info"), captured.clone());
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
