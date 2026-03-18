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
use std::time::Instant;

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
    ) -> Self {
        Self {
            compiler: Compiler::new(),
            app_dir: app_dir.to_path_buf(),
            output_dir: output_dir.to_path_buf(),
            project_name: project_name.to_string(),
            backend_port,
            tx,
            server_process: None,
        }
    }

    /// Run the full build pipeline (initial build).
    ///
    /// Returns the discovered routes on success, or errors on failure.
    pub fn initial_build(&mut self) -> Result<Vec<router::DiscoveredRoute>, Vec<DevError>> {
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
            .generate(&self.project_name, &self.output_dir, None)
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

        // Step 5: CSS generation (non-fatal)
        let project_dir = self.app_dir.parent().unwrap_or(Path::new("."));
        let _ = self
            .compiler
            .generate_css(project_dir, &self.app_dir, &self.output_dir, false);

        // Step 6: Cargo build
        self.cargo_build()?;

        // Step 7: Start server
        self.start_server();

        eprintln!("  Initial build completed in {:.0?}", start.elapsed());

        Ok(routes)
    }

    /// Handle a batch of file changes (incremental rebuild).
    pub fn handle_changes(&mut self, changes: Vec<ChangeKind>) -> RebuildResult {
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
            match self.full_rebuild() {
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
                .generate(&self.project_name, &self.output_dir, None)
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

            // Cargo build (incremental)
            if let Err(errors) = self.cargo_build() {
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

            // Restart server
            self.kill_server();
            self.start_server();
            let _ = self.tx.send(DevMessage::ErrorCleared);
            let _ = self.tx.send(DevMessage::Reload);
        } else if needs_css_only {
            // CSS-only change — regenerate Tailwind, no server restart
            let project_dir = self.app_dir.parent().unwrap_or(Path::new("."));
            let _ = self
                .compiler
                .generate_css(project_dir, &self.app_dir, &self.output_dir, false);
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
    fn full_rebuild(&mut self) -> Result<(), Vec<DevError>> {
        self.kill_server();
        self.compiler = Compiler::new();
        self.initial_build()?;
        let _ = self.tx.send(DevMessage::ErrorCleared);
        let _ = self.tx.send(DevMessage::Reload);
        Ok(())
    }

    /// Load all source files referenced by the routes into the compiler.
    fn load_route_files(&mut self, routes: &[router::DiscoveredRoute]) {
        for route in routes {
            let _ = self.compiler.add_file(&route.page_file);
            for layout in &route.layouts {
                let _ = self.compiler.add_file(layout);
            }
            for guard in &route.guards {
                let _ = self.compiler.add_file(guard);
            }
            for mw in &route.middleware {
                let _ = self.compiler.add_file(mw);
            }
        }
    }

    /// Run `cargo build` in the output directory.
    fn cargo_build(&self) -> Result<(), Vec<DevError>> {
        let status = Command::new("cargo")
            .arg("build")
            .current_dir(&self.output_dir)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status();

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

        match Command::new(&binary)
            .env("GALE_PORT", self.backend_port.to_string())
            .env("GALE_BIND", "127.0.0.1")
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
    fn format_parse_errors(&self) -> Vec<DevError> {
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
}

impl Drop for RebuildManager {
    fn drop(&mut self) {
        self.kill_server();
    }
}
