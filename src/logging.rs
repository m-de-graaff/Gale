use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::net::SocketAddr;
use std::sync::Mutex;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use axum::extract::ConnectInfo;
use axum::middleware::Next;
use axum::response::Response;
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::fmt::format::{self, FormatEvent, FormatFields};
use tracing_subscriber::fmt::FmtContext;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use crate::config::LoggingConfig;

pub fn init(config: &LoggingConfig) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.level));

    match (config.format.as_str(), config.output.as_str()) {
        ("json", "file") => {
            let file = open_log_file(&config.file_path);
            tracing_subscriber::registry()
                .with(filter)
                .with(
                    tracing_subscriber::fmt::layer()
                        .json()
                        .with_writer(file),
                )
                .init();
        }
        ("json", _) => {
            tracing_subscriber::registry()
                .with(filter)
                .with(tracing_subscriber::fmt::layer().json())
                .init();
        }
        (_, "file") => {
            let file = open_log_file(&config.file_path);
            tracing_subscriber::registry()
                .with(filter)
                .with(
                    tracing_subscriber::fmt::layer()
                        .event_format(ClfFormatter)
                        .with_writer(file),
                )
                .init();
        }
        _ => {
            tracing_subscriber::registry()
                .with(filter)
                .with(tracing_subscriber::fmt::layer().event_format(ClfFormatter))
                .init();
        }
    }
}

fn open_log_file(path: &str) -> Mutex<File> {
    if path.is_empty() {
        eprintln!("Fatal: log output set to 'file' but no file_path configured");
        std::process::exit(1);
    }

    let path = std::path::Path::new(path);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).unwrap_or_else(|e| {
                eprintln!(
                    "Fatal: cannot create log directory '{}': {e}",
                    parent.display()
                );
                std::process::exit(1);
            });
        }
    }

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .unwrap_or_else(|e| {
            eprintln!("Fatal: cannot open log file '{}': {e}", path.display());
            std::process::exit(1);
        });

    Mutex::new(file)
}

struct ClfFormatter;

impl<S, N> FormatEvent<S, N> for ClfFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: format::Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        if event.metadata().target() == "gale::request" {
            let mut visitor = ClfVisitor::default();
            event.record(&mut visitor);
            writeln!(
                writer,
                "{} - - [{}] \"{} {} {}\" {} {}",
                visitor.client_ip.as_deref().unwrap_or("-"),
                format_clf_timestamp(),
                visitor.method.as_deref().unwrap_or("-"),
                visitor.path.as_deref().unwrap_or("-"),
                visitor.version.as_deref().unwrap_or("-"),
                visitor.status.unwrap_or(0),
                visitor.bytes.unwrap_or(0),
            )
        } else {
            let timestamp = format_clf_timestamp();
            let level = event.metadata().level();
            let target = event.metadata().target();
            write!(writer, "{timestamp} {level} {target}: ")?;
            ctx.field_format().format_fields(writer.by_ref(), event)?;
            writeln!(writer)
        }
    }
}

#[derive(Default)]
struct ClfVisitor {
    client_ip: Option<String>,
    method: Option<String>,
    path: Option<String>,
    version: Option<String>,
    status: Option<u16>,
    bytes: Option<u64>,
    #[allow(dead_code)]
    duration_ms: Option<f64>,
    #[allow(dead_code)]
    message: Option<String>,
}

impl Visit for ClfVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        let s = format!("{:?}", value);
        match field.name() {
            "client_ip" => self.client_ip = Some(s),
            "method" => self.method = Some(s),
            "path" => self.path = Some(s),
            "version" => self.version = Some(s),
            "message" => self.message = Some(s),
            _ => {}
        }
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        match field.name() {
            "status" => self.status = Some(value as u16),
            "bytes" => self.bytes = Some(value),
            _ => {}
        }
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        if field.name() == "duration_ms" {
            self.duration_ms = Some(value);
        }
    }
}

pub fn format_clf_timestamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let days = (secs / 86400) as i64;
    let day_secs = (secs % 86400) as u32;
    let (year, month, day) = days_to_civil(days);

    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun",
        "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];

    format!(
        "{:02}/{}/{:04}:{:02}:{:02}:{:02} +0000",
        day,
        MONTHS[(month - 1) as usize],
        year,
        day_secs / 3600,
        (day_secs % 3600) / 60,
        day_secs % 60,
    )
}

/// Hinnant's days_to_civil algorithm.
/// Converts Unix epoch days to (year, month, day).
pub fn days_to_civil(unix_days: i64) -> (i64, u32, u32) {
    let z = unix_days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

pub async fn request_logging_middleware(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    req: axum::extract::Request,
    next: Next,
) -> Response {
    let start = Instant::now();
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    let version = format!("{:?}", req.version());

    let client_ip = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| peer.ip().to_string());

    let response = next.run(req).await;

    let status = response.status().as_u16();
    let bytes = response
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);
    let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

    tracing::info!(
        target: "gale::request",
        %client_ip,
        %method,
        %path,
        %version,
        status,
        bytes,
        duration_ms,
    );

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days_to_civil_unix_epoch() {
        let (y, m, d) = days_to_civil(0);
        assert_eq!((y, m, d), (1970, 1, 1));
    }

    #[test]
    fn days_to_civil_known_date() {
        // 2026-03-16 is 20528 days from Unix epoch
        let (y, m, d) = days_to_civil(20528);
        assert_eq!((y, m, d), (2026, 3, 16));
    }

    #[test]
    fn clf_timestamp_format() {
        let ts = format_clf_timestamp();
        // Should match pattern: DD/Mon/YYYY:HH:MM:SS +0000
        assert!(ts.ends_with("+0000"));
        assert_eq!(ts.len(), 26);
    }
}
