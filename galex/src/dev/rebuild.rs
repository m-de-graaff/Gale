//! Incremental rebuild manager and server process lifecycle.
//!
//! Coordinates the recompile pipeline when files change:
//! 1. Invalidate changed files in the compiler
//! 2. Re-parse all sources
//! 3. Type-check the merged program
//! 4. Regenerate Rust + JS code
//! 5. Run `cargo build` (incremental)
//! 6. Kill old server process, start new one

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use tokio::sync::broadcast;

use crate::compiler::Compiler;
use crate::router;

use super::server::{DevError, DevMessage};
use super::watcher::ChangeKind;

/// Manages incremental rebuilds and the generated server process.
pub struct RebuildManager {
    pub compiler: Compiler,
    pub app_dir: PathBuf,
    pub output_dir: PathBuf,
    pub project_name: String,
    pub backend_port: u16,
    pub tx: broadcast::Sender<DevMessage>,
    /// Shared with the dev WebSocket server — set when a Reload message
    /// may have been missed by disconnected clients.
    pub pending_reload: std::sync::Arc<std::sync::atomic::AtomicBool>,
    server_process: Option<Child>,
}

/// Result of a rebuild attempt.
pub struct RebuildResult {
    pub success: bool,
    pub duration: std::time::Duration,
    pub changed_files: Vec<PathBuf>,
    pub errors: Vec<DevError>,
}

impl RebuildManager {
    /// Create a new rebuild manager.
    pub fn new(
        app_dir: &Path,
        output_dir: &Path,
        project_name: &str,
        backend_port: u16,
        tx: broadcast::Sender<DevMessage>,
        pending_reload: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> Self {
        Self {
            compiler: Compiler::new(),
            app_dir: app_dir.to_path_buf(),
            output_dir: output_dir.to_path_buf(),
            project_name: project_name.to_string(),
            backend_port,
            tx,
            pending_reload,
            server_process: None,
        }
    }

    /// Run the full build pipeline (initial build).
    ///
    /// Returns the discovered routes on success, or errors on failure.
    pub async fn initial_build(&mut self) -> Result<Vec<router::DiscoveredRoute>, Vec<DevError>> {
        let start = Instant::now();

        // Step 1: Discover routes
        let routes = router::discovery::discover_routes(&self.app_dir).map_err(|errs| {
            errs.iter()
                .map(|e| DevError {
                    file: e
                        .path
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_default(),
                    line: 0,
                    col: 0,
                    message: e.message.clone(),
                    code: None,
                    source_line: None,
                    suggestion: None,
                })
                .collect::<Vec<_>>()
        })?;

        // Step 2: Load and parse all source files
        self.load_route_files(&routes);
        let parse_errors = self.compiler.parse_all();
        if parse_errors > 0 {
            return Err(self.format_parse_errors());
        }

        // Step 3: Type check
        let type_errors = self.compiler.check();
        if !type_errors.is_empty() {
            return Err(self.format_type_errors(&type_errors));
        }

        // Step 4: Code generation
        self.compiler.set_routes(routes.clone());
        self.compiler
            .generate(&self.project_name, &self.output_dir, None, true)
            .map_err(|e| {
                vec![DevError {
                    file: String::new(),
                    line: 0,
                    col: 0,
                    message: format!("Code generation failed: {e}"),
                    code: None,
                    source_line: None,
                    suggestion: None,
                }]
            })?;

        // Step 5: CSS generation — report errors instead of discarding
        let project_dir = self.app_dir.parent().unwrap_or(Path::new("."));
        if let Err(e) =
            self.compiler
                .generate_css(project_dir, &self.app_dir, &self.output_dir, false)
        {
            eprintln!("  warning: CSS generation failed: {e}");
            eprintln!("  (Tailwind may not be available — styles will be missing)");
        }

        // Step 6: Cargo build (async — doesn't block the proxy/WebSocket)
        self.cargo_build().await?;

        // Step 7: Start server and wait for it to accept connections
        self.start_server();
        self.wait_for_server_ready().await;

        eprintln!("  Initial build completed in {:.0?}", start.elapsed());

        Ok(routes)
    }

    /// Handle a batch of file changes (incremental rebuild).
    pub async fn handle_changes(&mut self, changes: Vec<ChangeKind>) -> RebuildResult {
        let start = Instant::now();
        let mut changed_files = Vec::new();
        let mut needs_rediscover = false;
        let mut needs_gx_rebuild = false;
        let mut needs_css_only = false;
        let mut needs_full_restart = false;

        for change in &changes {
            match change {
                ChangeKind::ConfigChanged => needs_full_restart = true,
                ChangeKind::GxStructural(p) => {
                    needs_rediscover = true;
                    needs_gx_rebuild = true;
                    changed_files.push(p.clone());
                }
                ChangeKind::GxModified(p) => {
                    needs_gx_rebuild = true;
                    changed_files.push(p.clone());
                }
                ChangeKind::CssChanged(_) => needs_css_only = true,
                ChangeKind::AssetChanged(p) => {
                    // Notify browser of asset change
                    let rel_path = p
                        .strip_prefix(self.app_dir.parent().unwrap_or(Path::new(".")))
                        .unwrap_or(p)
                        .display()
                        .to_string();
                    let _ = self.tx.send(DevMessage::AssetReload { path: rel_path });
                }
            }
        }

        if needs_full_restart {
            match self.full_rebuild().await {
                Ok(_) => {
                    return RebuildResult {
                        success: true,
                        duration: start.elapsed(),
                        changed_files,
                        errors: vec![],
                    }
                }
                Err(errors) => {
                    return RebuildResult {
                        success: false,
                        duration: start.elapsed(),
                        changed_files,
                        errors,
                    }
                }
            }
        }

        if needs_gx_rebuild {
            // Re-discover routes if structural change
            if needs_rediscover {
                match router::discovery::discover_routes(&self.app_dir) {
                    Ok(routes) => {
                        // Reload all files for new routes
                        self.compiler = Compiler::new();
                        self.load_route_files(&routes);
                        self.compiler.set_routes(routes);
                    }
                    Err(errs) => {
                        let errors: Vec<DevError> = errs
                            .iter()
                            .map(|e| DevError {
                                file: e
                                    .path
                                    .as_ref()
                                    .map(|p| p.display().to_string())
                                    .unwrap_or_default(),
                                line: 0,
                                col: 0,
                                message: e.message.clone(),
                                code: None,
                                source_line: None,
                                suggestion: None,
                            })
                            .collect();
                        let _ = self.tx.send(DevMessage::Error {
                            errors: errors.clone(),
                        });
                        return RebuildResult {
                            success: false,
                            duration: start.elapsed(),
                            changed_files,
                            errors,
                        };
                    }
                }
            } else {
                // Incremental: invalidate only changed files
                self.compiler.invalidate_files(&changed_files);
            }

            // Parse
            let parse_err_count = self.compiler.parse_all();
            if parse_err_count > 0 {
                let errors = self.format_parse_errors();
                let _ = self.tx.send(DevMessage::Error {
                    errors: errors.clone(),
                });
                return RebuildResult {
                    success: false,
                    duration: start.elapsed(),
                    changed_files,
                    errors,
                };
            }

            // Type check
            let type_errors = self.compiler.check();
            if !type_errors.is_empty() {
                let errors = self.format_type_errors(&type_errors);
                let _ = self.tx.send(DevMessage::Error {
                    errors: errors.clone(),
                });
                return RebuildResult {
                    success: false,
                    duration: start.elapsed(),
                    changed_files,
                    errors,
                };
            }

            // Codegen
            if let Err(e) = self
                .compiler
                .generate(&self.project_name, &self.output_dir, None, true)
            {
                let errors = vec![DevError {
                    file: String::new(),
                    line: 0,
                    col: 0,
                    message: format!("Codegen failed: {e}"),
                    code: None,
                    source_line: None,
                    suggestion: None,
                }];
                let _ = self.tx.send(DevMessage::Error {
                    errors: errors.clone(),
                });
                return RebuildResult {
                    success: false,
                    duration: start.elapsed(),
                    changed_files,
                    errors,
                };
            }

            // CSS generation
            let project_dir = self.app_dir.parent().unwrap_or(Path::new("."));
            if let Err(e) =
                self.compiler
                    .generate_css(project_dir, &self.app_dir, &self.output_dir, true)
            {
                eprintln!("  warning: CSS generation failed: {e}");
            }

            // Cargo build (incremental, async)
            if let Err(errors) = self.cargo_build().await {
                let _ = self.tx.send(DevMessage::Error {
                    errors: errors.clone(),
                });
                return RebuildResult {
                    success: false,
                    duration: start.elapsed(),
                    changed_files,
                    errors,
                };
            }

            // Restart server and wait for it to be ready
            self.kill_server();
            self.start_server();
            self.wait_for_server_ready().await;
            let _ = self.tx.send(DevMessage::ErrorCleared);
            // Set the pending flag so late-connecting WebSocket clients
            // (whose connection dropped during the rebuild) still reload.
            self.pending_reload
                .store(true, std::sync::atomic::Ordering::SeqCst);
            let _ = self.tx.send(DevMessage::Reload);
        } else if needs_css_only {
            // CSS-only change — regenerate Tailwind, no server restart
            let project_dir = self.app_dir.parent().unwrap_or(Path::new("."));
            if let Err(e) =
                self.compiler
                    .generate_css(project_dir, &self.app_dir, &self.output_dir, false)
            {
                eprintln!("  warning: CSS generation failed: {e}");
            }
            let _ = self.tx.send(DevMessage::CssReload);
        }

        RebuildResult {
            success: true,
            duration: start.elapsed(),
            changed_files,
            errors: vec![],
        }
    }

    /// Full rebuild (for config changes).
    async fn full_rebuild(&mut self) -> Result<(), Vec<DevError>> {
        self.kill_server();
        self.compiler = Compiler::new();
        self.initial_build().await?;
        let _ = self.tx.send(DevMessage::ErrorCleared);
        self.pending_reload
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let _ = self.tx.send(DevMessage::Reload);
        Ok(())
    }

    /// Load all source files referenced by the routes into the compiler.
    ///
    /// Deduplication is handled by `Compiler::add_file_dedup` which
    /// tracks canonicalized paths internally.
    fn load_route_files(&mut self, routes: &[router::DiscoveredRoute]) {
        for route in routes {
            let files = std::iter::once(&route.page_file)
                .chain(route.layouts.iter())
                .chain(route.guards.iter())
                .chain(route.middleware.iter());

            for path in files {
                let _ = self.compiler.add_file_dedup(path);
            }
        }
    }

    /// Run `cargo build` in the output directory (async — does not block
    /// the tokio runtime, so the dev proxy and WebSocket hub stay alive).
    async fn cargo_build(&self) -> Result<(), Vec<DevError>> {
        let status = tokio::process::Command::new("cargo")
            .arg("build")
            .current_dir(&self.output_dir)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await;

        match status {
            Ok(s) if s.success() => Ok(()),
            Ok(s) => Err(vec![DevError {
                file: String::new(),
                line: 0,
                col: 0,
                message: format!("cargo build failed (exit code: {s})"),
                code: None,
                source_line: None,
                suggestion: Some("Check the terminal output for Rust compiler errors".into()),
            }]),
            Err(e) => Err(vec![DevError {
                file: String::new(),
                line: 0,
                col: 0,
                message: format!("Failed to run cargo: {e}"),
                code: None,
                source_line: None,
                suggestion: Some("Ensure Rust toolchain is installed".into()),
            }]),
        }
    }

    /// Wait for the backend server to accept connections on its port.
    ///
    /// Polls every 50ms, gives up after 15 seconds.  This ensures the
    /// browser reload doesn't hit a 502 because the server hasn't bound
    /// its port yet.
    async fn wait_for_server_ready(&self) {
        let addr = format!("127.0.0.1:{}", self.backend_port);
        let deadline = Instant::now() + Duration::from_secs(15);
        while Instant::now() < deadline {
            if tokio::net::TcpStream::connect(&addr).await.is_ok() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        eprintln!(
            "  warning: backend server did not become ready on port {}",
            self.backend_port
        );
    }

    /// Start the generated server as a child process.
    fn start_server(&mut self) {
        let binary_name = if cfg!(windows) {
            format!("{}.exe", self.project_name)
        } else {
            self.project_name.clone()
        };
        let binary = self
            .output_dir
            .join("target")
            .join("debug")
            .join(&binary_name);

        if !binary.exists() {
            eprintln!("  warning: binary not found at {}", binary.display());
            return;
        }

        // Point GALE_ROOT at the generated public/ directory so the
        // backend's ServeDir finds CSS/JS assets.  Without this, the
        // backend resolves "./public" from the user's CWD, which is the
        // project root — not .gale_dev/ where the assets live.
        let public_root = self.output_dir.join("public");

        match Command::new(&binary)
            .env("GALE_PORT", self.backend_port.to_string())
            .env("GALE_BIND", "127.0.0.1")
            .env("GALE_ROOT", public_root.display().to_string())
            // Disable compression on the dev backend.  The dev proxy
            // (port 3000) forwards responses via reqwest which does NOT
            // have decompression support (default-features = false).
            // If the backend compresses, the proxy passes through raw
            // brotli/gzip bytes → garbled CSS/JS in the browser.
            .env("GALE_COMPRESSION_ENABLED", "false")
            // Relax CSP for dev mode — inline <script> tags are used by
            // form wiring and hydration.  Production builds should use
            // a strict CSP with nonces.
            .env("GALE_CSP", "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'")
            // Keep request logs (info level) visible while suppressing
            // the startup "listening addr=..." message (logged at debug
            // after our change to server.rs).
            .env("GALE_LOG_LEVEL", "info")
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
        {
            Ok(child) => {
                self.server_process = Some(child);
            }
            Err(e) => {
                eprintln!("  warning: failed to start server: {e}");
            }
        }
    }

    /// Kill the running server process.
    pub fn kill_server(&mut self) {
        if let Some(ref mut child) = self.server_process {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.server_process = None;
    }

    /// Format parse errors into DevError structs.
    ///
    /// Uses the raw lex/parse errors when available to include structured
    /// error codes, or falls back to formatted strings.
    fn format_parse_errors(&self) -> Vec<DevError> {
        let diagnostics = self.compiler.parse_diagnostics();
        if !diagnostics.is_empty() {
            return diagnostics
                .iter()
                .map(|d| self.diagnostic_to_dev_error(d))
                .collect();
        }
        // Fallback to formatted strings
        self.compiler
            .parse_errors
            .iter()
            .map(|msg| DevError {
                file: String::new(),
                line: 0,
                col: 0,
                message: msg.clone(),
                code: None,
                source_line: None,
                suggestion: None,
            })
            .collect()
    }

    /// Format type errors into DevError structs.
    fn format_type_errors(&self, errors: &[String]) -> Vec<DevError> {
        errors
            .iter()
            .map(|msg| DevError {
                file: String::new(),
                line: 0,
                col: 0,
                message: msg.clone(),
                code: None,
                source_line: None,
                suggestion: None,
            })
            .collect()
    }

    /// Convert a unified [`Diagnostic`](crate::errors::Diagnostic) to a [`DevError`].
    fn diagnostic_to_dev_error(&self, d: &crate::errors::Diagnostic) -> DevError {
        let file = self
            .compiler
            .file_table
            .get_path(d.span.file_id)
            .map(|p| p.display().to_string())
            .unwrap_or_default();

        let source_line = self.compiler.sources.get(&d.span.file_id).and_then(|src| {
            src.lines()
                .nth(d.span.line.saturating_sub(1) as usize)
                .map(|l| l.to_string())
        });

        DevError {
            file,
            line: d.span.line,
            col: d.span.col,
            message: d.message.clone(),
            code: Some(d.code.as_str()),
            source_line,
            suggestion: d.help.clone().or_else(|| d.hint.clone()),
        }
    }
}

impl Drop for RebuildManager {
    fn drop(&mut self) {
        self.kill_server();
    }
}
