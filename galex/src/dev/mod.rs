//! Development server with hot reload.
//!
//! `gale dev` runs this module, which:
//! 1. Performs an initial build of the project
//! 2. Starts the generated server as a child process
//! 3. Starts a dev proxy server on the requested port
//! 4. Watches for file changes and incrementally rebuilds
//! 5. Sends reload/error signals to connected browsers via WebSocket

pub mod rebuild;
pub mod server;
pub mod watcher;

use std::path::Path;
use std::time::Instant;

use tokio::sync::broadcast;

use crate::router::DiscoveredRoute;
use server::DevMessage;

/// Run the full dev server pipeline.
///
/// This is the main entry point for `gale dev`. It blocks until Ctrl+C.
pub async fn run_dev_server(
    app_dir: &Path,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let project_dir = app_dir.parent().unwrap_or(Path::new("."));
    let output_dir = project_dir.join(".gale_dev");
    let backend_port = find_available_port(port + 1);

    // Broadcast channel for dev messages (browser notifications)
    let (tx, _) = broadcast::channel::<DevMessage>(64);

    // ── Initial build ──────────────────────────────────────────
    eprintln!();
    eprintln!("  Gale dev server");
    eprintln!();

    let mut manager = rebuild::RebuildManager::new(
        app_dir,
        &output_dir,
        "gale_dev_app",
        backend_port,
        tx.clone(),
    );

    let start = Instant::now();
    eprintln!("  Building...");
    let routes = match manager.initial_build() {
        Ok(routes) => {
            let _ = tx.send(DevMessage::ErrorCleared);
            routes
        }
        Err(errors) => {
            print_error_count(errors.len());
            for err in &errors {
                eprintln!("    {}", err.message);
            }
            let _ = tx.send(DevMessage::Error {
                errors: errors.clone(),
            });
            Vec::new()
        }
    };
    print_ready(port, &routes, start.elapsed());

    // ── Start dev proxy server ─────────────────────────────────
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        server::run_dev_server(port, backend_port, tx_clone).await;
    });

    // ── Start file watcher ─────────────────────────────────────
    let mut file_watcher = match watcher::FileWatcher::start(app_dir, project_dir) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("  warning: file watcher failed to start: {e}");
            eprintln!("  (changes will not be detected automatically)");
            // Still run the server — just without file watching
            tokio::signal::ctrl_c().await?;
            manager.kill_server();
            return Ok(());
        }
    };

    // ── Event loop ─────────────────────────────────────────────
    loop {
        tokio::select! {
            Some(changes) = file_watcher.next_changes() => {
                let start = Instant::now();
                let summary = summarize_changes(&changes);
                eprintln!();
                eprintln!("  {summary}");

                let result = manager.handle_changes(changes);

                if result.success {
                    eprintln!(
                        "  Rebuilt in {:.0?}",
                        result.duration,
                    );
                } else {
                    print_error_count(result.errors.len());
                    for err in &result.errors {
                        eprintln!("    {}", err.message);
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                eprintln!();
                eprintln!("  Shutting down...");
                manager.kill_server();
                break;
            }
        }
    }

    Ok(())
}

// ── Terminal output helpers ────────────────────────────────────────────

/// Print the startup ready message with route table.
fn print_ready(port: u16, routes: &[DiscoveredRoute], duration: std::time::Duration) {
    eprintln!();
    eprintln!("  Ready in {:.0?}", duration);
    eprintln!();
    eprintln!(
        "  Local:   http://localhost:{port}"
    );
    eprintln!();

    if !routes.is_empty() {
        eprintln!("  Routes:");
        for route in routes {
            eprintln!(
                "    {:20} {}",
                route.url_path,
                route.page_file.display()
            );
        }
        eprintln!();
    }

    eprintln!("  Watching for changes...");
    eprintln!();
}

/// Print an error count summary.
fn print_error_count(count: usize) {
    eprintln!(
        "  Found {} error{}",
        count,
        if count != 1 { "s" } else { "" }
    );
}

/// Summarize a batch of changes for terminal display.
fn summarize_changes(changes: &[watcher::ChangeKind]) -> String {
    let gx_count = changes
        .iter()
        .filter(|c| {
            matches!(
                c,
                watcher::ChangeKind::GxModified(_) | watcher::ChangeKind::GxStructural(_)
            )
        })
        .count();
    let css_count = changes
        .iter()
        .filter(|c| matches!(c, watcher::ChangeKind::CssChanged(_)))
        .count();
    let asset_count = changes
        .iter()
        .filter(|c| matches!(c, watcher::ChangeKind::AssetChanged(_)))
        .count();
    let config = changes
        .iter()
        .any(|c| matches!(c, watcher::ChangeKind::ConfigChanged));

    let mut parts = Vec::new();
    if config {
        parts.push("config changed — full restart".to_string());
    }
    if gx_count > 0 {
        parts.push(format!(
            "{gx_count} .gx file{} changed",
            if gx_count != 1 { "s" } else { "" }
        ));
    }
    if css_count > 0 {
        parts.push(format!(
            "{css_count} CSS file{} changed",
            if css_count != 1 { "s" } else { "" }
        ));
    }
    if asset_count > 0 {
        parts.push(format!(
            "{asset_count} asset{} changed",
            if asset_count != 1 { "s" } else { "" }
        ));
    }

    if parts.is_empty() {
        "File changed".to_string()
    } else {
        parts.join(", ")
    }
}

/// Find the first available port starting from `start`.
fn find_available_port(start: u16) -> u16 {
    for port in start..start + 10 {
        if std::net::TcpListener::bind(("127.0.0.1", port)).is_ok() {
            return port;
        }
    }
    start // Fallback
}
