use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::net::SocketAddr;
use std::sync::Mutex;
use std::time::Instant;

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
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.level));

    match (config.format.as_str(), config.output.as_str()) {
        ("json", "file") => {
            let file = open_log_file(&config.file_path);
            tracing_subscriber::registry()
                .with(filter)
                .with(tracing_subscriber::fmt::layer().json().with_writer(file))
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
                        .event_format(CompactFormatter)
                        .with_writer(file),
                )
                .init();
        }
        _ => {
            tracing_subscriber::registry()
                .with(filter)
                .with(
                    tracing_subscriber::fmt::layer()
                        .event_format(CompactFormatter)
                        .with_ansi(false),
                )
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

/// Compact request formatter inspired by Next.js dev output.
///
/// Request lines:  ` GET /about 200 in 4ms`
/// Non-request events use a simple `LEVEL target: message` format.
struct CompactFormatter;

impl<S, N> FormatEvent<S, N> for CompactFormatter
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
            let mut visitor = RequestVisitor::default();
            event.record(&mut visitor);

            let method = visitor.method.as_deref().unwrap_or("-");
            let path = visitor.path.as_deref().unwrap_or("/");
            let status = visitor.status.unwrap_or(0);
            let duration = visitor.duration_ms.unwrap_or(0.0);

            // Format duration: <1ms → "<1ms", otherwise round
            let dur_str = if duration < 1.0 {
                "<1ms".to_string()
            } else {
                format!("{}ms", duration.round() as u64)
            };

            writeln!(writer, " {method} {path} {status} in {dur_str}")
        } else {
            let level = event.metadata().level();
            let target = event.metadata().target();
            write!(writer, " {level} {target}: ")?;
            ctx.field_format().format_fields(writer.by_ref(), event)?;
            writeln!(writer)
        }
    }
}

#[derive(Default)]
struct RequestVisitor {
    method: Option<String>,
    path: Option<String>,
    status: Option<u16>,
    duration_ms: Option<f64>,
}

impl Visit for RequestVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        let s = format!("{:?}", value);
        match field.name() {
            "method" => self.method = Some(s),
            "path" => self.path = Some(s),
            _ => {}
        }
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        if field.name() == "status" {
            self.status = Some(value as u16);
        }
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        if field.name() == "duration_ms" {
            self.duration_ms = Some(value);
        }
    }
}

/// Static asset path prefixes and extensions to suppress in dev logging.
///
/// These are silenced because they add noise without useful signal
/// during development — the user cares about page and API routes.
fn is_static_asset(path: &str) -> bool {
    if path.starts_with("/_gale/")
        || path.starts_with("/_next/")
        || path.starts_with("/public/")
        || path.starts_with("/static/")
        || path.starts_with("/favicon")
    {
        return true;
    }
    // Check common static file extensions
    matches!(
        path.rsplit('.').next(),
        Some(
            "js" | "css"
                | "map"
                | "ico"
                | "png"
                | "jpg"
                | "jpeg"
                | "gif"
                | "svg"
                | "webp"
                | "avif"
                | "woff"
                | "woff2"
                | "ttf"
                | "otf"
                | "eot"
        )
    )
}

pub async fn request_logging_middleware(
    ConnectInfo(_peer): ConnectInfo<SocketAddr>,
    req: axum::extract::Request,
    next: Next,
) -> Response {
    let start = Instant::now();
    let method = req.method().to_string();
    let path = req.uri().path().to_string();

    let response = next.run(req).await;

    // Suppress static asset requests for cleaner dev output.
    if is_static_asset(&path) {
        return response;
    }

    let status = response.status().as_u16();
    let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

    tracing::info!(
        target: "gale::request",
        %method,
        %path,
        status,
        duration_ms,
    );

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_asset_detection() {
        assert!(is_static_asset("/_gale/runtime.js"));
        assert!(is_static_asset("/static/logo.png"));
        assert!(is_static_asset("/favicon.ico"));
        assert!(is_static_asset("/styles/app.css"));
        assert!(is_static_asset("/fonts/inter.woff2"));
        assert!(!is_static_asset("/"));
        assert!(!is_static_asset("/about"));
        assert!(!is_static_asset("/api/users"));
        assert!(!is_static_asset("/dashboard"));
    }
}
