//! Compiler driver — coordinates the full .gx → binary pipeline.
//!
//! Owns the file table, source texts, parsed programs, and orchestrates
//! lexing → parsing → type checking → code generation.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::ast::Program;
use crate::checker::TypeChecker;
use crate::parser::{self, ParseResult};
use crate::router::DiscoveredRoute;
use crate::span::{FileTable, Span};
use crate::types::ty::TypeInterner;

/// The GaleX compiler driver.
///
/// Coordinates multi-file compilation: file loading, parsing, type checking,
/// and code generation.
pub struct Compiler {
    /// Maps file IDs to source file paths.
    pub file_table: FileTable,
    /// Maps file IDs to source text (kept for error reporting).
    pub sources: HashMap<u32, String>,
    /// Parsed programs indexed by file ID.
    pub programs: Vec<(u32, Program)>,
    /// Parse errors accumulated from all files.
    pub parse_errors: Vec<String>,
    /// Discovered routes (populated by `set_routes`).
    pub routes: Vec<DiscoveredRoute>,
}

impl Compiler {
    /// Create a new empty compiler.
    pub fn new() -> Self {
        Self {
            file_table: FileTable::new(),
            sources: HashMap::new(),
            programs: Vec::new(),
            parse_errors: Vec::new(),
            routes: Vec::new(),
        }
    }

    /// Add a source file to the compiler. Returns the file ID.
    pub fn add_file(&mut self, path: &Path) -> Result<u32, std::io::Error> {
        let source = std::fs::read_to_string(path)?;
        let file_id = self.file_table.add_file(path.to_path_buf());
        self.sources.insert(file_id, source);
        Ok(file_id)
    }

    /// Parse all loaded source files.
    ///
    /// Returns the number of parse errors encountered.
    pub fn parse_all(&mut self) -> usize {
        self.programs.clear();
        self.parse_errors.clear();

        let file_ids: Vec<u32> = self.sources.keys().copied().collect();
        for file_id in file_ids {
            let source = &self.sources[&file_id];
            let result = parser::parse(source, file_id);
            for err in &result.lex_errors {
                let loc = self.file_table.format_span(&Span::dummy());
                self.parse_errors.push(format!("{loc}: {err}"));
            }
            for err in &result.parse_errors {
                let loc = self.file_table.format_span(&err.span);
                self.parse_errors.push(format!("{loc}: {err}"));
            }
            self.programs.push((file_id, result.program));
        }

        self.parse_errors.len()
    }

    /// Merge all parsed programs into a single program for type checking.
    ///
    /// Items from all files are concatenated. The merged program's span
    /// uses file_id 0 (the first file, or dummy).
    pub fn merge_programs(&self) -> Program {
        let mut all_items = Vec::new();
        for (_, program) in &self.programs {
            all_items.extend(program.items.clone());
        }
        Program {
            items: all_items,
            span: Span::dummy(),
        }
    }

    /// Type-check the merged program.
    ///
    /// Returns type errors as formatted strings.
    pub fn check(&self) -> Vec<String> {
        let merged = self.merge_programs();
        let mut checker = TypeChecker::new();
        let errors = checker.check_program(&merged);
        errors.iter().map(|e| format!("{e}")).collect()
    }

    /// Type-check and return raw error objects (for structured error display).
    ///
    /// Unlike [`check()`], this preserves the span, kind, and suggestion
    /// fields — useful for the dev server error overlay.
    pub fn check_raw(&self) -> Vec<crate::types::constraint::TypeError> {
        let merged = self.merge_programs();
        let mut checker = TypeChecker::new();
        checker.check_program(&merged)
    }

    /// Run code generation on the merged program.
    ///
    /// Writes the generated Rust + JS project to `output_dir`.
    /// `gale_crate_path` overrides the `gale = { path = "..." }` dependency
    /// in the generated Cargo.toml (default: `"../"`).
    pub fn generate(
        &self,
        project_name: &str,
        output_dir: &Path,
        gale_crate_path: Option<&str>,
    ) -> Result<(), std::io::Error> {
        let merged = self.merge_programs();
        let interner = TypeInterner::new();
        crate::codegen::generate(
            &merged,
            &interner,
            project_name,
            output_dir,
            gale_crate_path,
        )
    }

    /// Set the discovered routes (from filesystem walking).
    pub fn set_routes(&mut self, routes: Vec<DiscoveredRoute>) {
        self.routes = routes;
    }

    /// Generate Tailwind CSS from the project's class usage.
    ///
    /// Scans parsed programs for class names, generates a Tailwind config,
    /// and shells out to the Tailwind CLI. Writes output to
    /// `{output_dir}/public/_gale/styles.css`.
    ///
    /// Returns `Ok(true)` if CSS was generated, `Ok(false)` if Tailwind
    /// is disabled, or `Err` on failure.
    pub fn generate_css(
        &self,
        project_dir: &Path,
        app_dir: &Path,
        output_dir: &Path,
        minify: bool,
    ) -> Result<bool, String> {
        let tw_config = crate::tailwind::config::load_config(project_dir);
        if !tw_config.enabled {
            return Ok(false);
        }

        let safelist = crate::tailwind::extract::generate_safelist(&self.programs);
        let output_css = output_dir.join("public/_gale/styles.css");

        crate::tailwind::generate::run_tailwind_cli(
            &tw_config,
            app_dir,
            &safelist,
            &output_css,
            minify,
        )
        .map(|_| true)
        .map_err(|e| e.to_string())
    }

    /// Load all installed packages from `gale_modules/`.
    ///
    /// Scans the `gale_modules/` directory for installed packages,
    /// reads their manifests, and adds their `.gx` files to the compiler.
    pub fn load_packages(&mut self, project_dir: &Path) -> Vec<String> {
        let modules_dir = project_dir.join("gale_modules");
        if !modules_dir.is_dir() {
            return vec![];
        }

        let mut errors = Vec::new();
        let entries = match std::fs::read_dir(&modules_dir) {
            Ok(e) => e,
            Err(e) => {
                errors.push(format!("failed to read gale_modules/: {e}"));
                return errors;
            }
        };

        for entry in entries.flatten() {
            let pkg_dir = entry.path();
            if !pkg_dir.is_dir() {
                continue;
            }

            // Check for gale-package.toml
            let manifest_path = pkg_dir.join("gale-package.toml");
            if !manifest_path.is_file() {
                continue;
            }

            // Add all .gx files from the package
            for gx_entry in walkdir::WalkDir::new(&pkg_dir)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if gx_entry.file_type().is_file()
                    && gx_entry.path().extension().and_then(|e| e.to_str()) == Some("gx")
                {
                    if let Err(e) = self.add_file(gx_entry.path()) {
                        errors.push(format!("failed to load {}: {e}", gx_entry.path().display()));
                    }
                }
            }
        }

        errors
    }

    /// Invalidate files that have changed (for incremental rebuilds).
    pub fn invalidate_files(&mut self, paths: &[PathBuf]) {
        // Remove programs for changed files
        self.programs.retain(|(fid, _)| {
            if let Some(p) = self.file_table.get_path(*fid) {
                !paths.iter().any(|changed| changed == p)
            } else {
                true
            }
        });
        // Re-read changed files
        for path in paths {
            if path.exists() {
                if let Ok(source) = std::fs::read_to_string(path) {
                    // Find existing file_id or add new
                    let file_id = self.file_table.add_file(path.clone());
                    self.sources.insert(file_id, source);
                }
            }
        }
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}
