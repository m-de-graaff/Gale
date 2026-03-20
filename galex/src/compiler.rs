//! Compiler driver — coordinates the full .gx → binary pipeline.
//!
//! Owns the file table, source texts, parsed programs, and orchestrates
//! lexing → parsing → type checking → code generation.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::ast::Program;
use crate::checker::TypeChecker;
use crate::error::LexError;
use crate::errors::{Diagnostic, IntoDiagnostic};
use crate::parser;
use crate::parser::error::ParseError;
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
    /// Parse errors accumulated from all files (formatted strings for backward compat).
    pub parse_errors: Vec<String>,
    /// Raw lex errors per file (file_id, errors).
    pub lex_errors: Vec<(u32, Vec<LexError>)>,
    /// Raw parse errors from all files.
    pub raw_parse_errors: Vec<ParseError>,
    /// Discovered routes (populated by `set_routes`).
    pub routes: Vec<DiscoveredRoute>,
    /// Canonicalized paths already added (for dedup via `add_file_dedup`).
    added_paths: HashSet<PathBuf>,
}

impl Compiler {
    /// Create a new empty compiler.
    pub fn new() -> Self {
        Self {
            file_table: FileTable::new(),
            sources: HashMap::new(),
            programs: Vec::new(),
            parse_errors: Vec::new(),
            lex_errors: Vec::new(),
            raw_parse_errors: Vec::new(),
            routes: Vec::new(),
            added_paths: HashSet::new(),
        }
    }

    /// Add a source file to the compiler. Returns the file ID.
    ///
    /// **Note:** This does NOT deduplicate — the same file can be added
    /// multiple times, resulting in duplicate parse/check work and false
    /// "already defined" errors.  Prefer [`add_file_dedup`] for route-
    /// discovery call sites where layouts are shared across routes.
    pub fn add_file(&mut self, path: &Path) -> Result<u32, std::io::Error> {
        let source = std::fs::read_to_string(path)?;
        let file_id = self.file_table.add_file(path.to_path_buf());
        self.sources.insert(file_id, source);
        Ok(file_id)
    }

    /// Add a source file, skipping it if the same canonical path was
    /// already added.  Returns `Ok(None)` when the file was skipped.
    pub fn add_file_dedup(&mut self, path: &Path) -> Result<Option<u32>, std::io::Error> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if !self.added_paths.insert(canonical) {
            return Ok(None); // already loaded
        }
        self.add_file(path).map(Some)
    }

    /// Parse all loaded source files.
    ///
    /// Returns the number of parse errors encountered.
    pub fn parse_all(&mut self) -> usize {
        self.programs.clear();
        self.parse_errors.clear();
        self.lex_errors.clear();
        self.raw_parse_errors.clear();

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
            // Store raw errors for structured diagnostic output
            if !result.lex_errors.is_empty() {
                self.lex_errors.push((file_id, result.lex_errors.clone()));
            }
            self.raw_parse_errors.extend(result.parse_errors.clone());

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

    /// Convert lex and parse errors from stored raw errors into Diagnostics.
    pub fn parse_diagnostics(&self) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        for (_file_id, lex_errs) in &self.lex_errors {
            for err in lex_errs {
                diagnostics.push(err.clone().into_diagnostic());
            }
        }

        for err in &self.raw_parse_errors {
            diagnostics.push(err.clone().into_diagnostic());
        }

        diagnostics
    }

    /// Run all validation phases and return unified diagnostics.
    ///
    /// This runs:
    /// 1. Type checking (produces TypeErrors -> converted to Diagnostics)
    /// 2. Lint rules (produces LintWarnings -> converted to Diagnostics)
    /// 3. Guard validation (GX0600-GX0632)
    /// 4. Import validation (GX0800-GX0809)
    /// 5. Action/query/channel validation (GX0900-GX0913)
    /// 6. Store validation (GX1000-GX1006)
    /// 7. Env validation (GX1100-GX1108)
    /// 8. Middleware validation (GX1300-GX1304)
    /// 9. Head/SEO validation (GX1400-GX1408)
    /// 10. Form validation (GX1500-GX1507)
    /// 11. Reactivity validation (GX1600-GX1610)
    pub fn check_all(&self) -> Vec<Diagnostic> {
        let merged = self.merge_programs();
        let mut diagnostics = Vec::new();

        // Phase 1: Type checking
        let mut checker = TypeChecker::new();
        let type_errors = checker.check_program(&merged);
        for err in type_errors {
            diagnostics.push(err.into_diagnostic());
        }

        // Collect template diagnostics from type checker
        diagnostics.append(&mut checker.template_diagnostics);

        // Phase 2: Lint rules
        let lint_warnings = crate::lint::lint_program(&merged);
        for warn in lint_warnings {
            diagnostics.push(warn.into_diagnostic());
        }

        // Phase 3: Guard validation (GX0600-GX0632)
        crate::checker::guard::validate_guards(&merged, &mut diagnostics);

        // Phase 4: Import validation (GX0800-GX0809)
        diagnostics.extend(crate::checker::imports::validate_imports(&merged));

        // Phase 5: Action/query/channel validation (GX0900-GX0913)
        diagnostics.extend(crate::checker::action::validate_actions(&merged));

        // Phase 6: Store validation (GX1000-GX1006)
        diagnostics.extend(crate::checker::store::validate_stores(&merged));

        // Phase 7: Env validation (GX1100-GX1108)
        diagnostics.extend(crate::checker::env::validate_env_decls(&merged));

        // Phase 8-10: head, form — already run during type checking or need component context
        // Phase 11: Reactivity validation (GX1600-GX1610)
        diagnostics.extend(crate::checker::reactivity::validate_reactivity(&merged));

        diagnostics
    }

    /// Run code generation on the merged program.
    ///
    /// Writes the generated Rust + JS project to `output_dir`.
    /// `gale_dep_override` overrides the gale dependency in the generated
    /// Cargo.toml. `dev_mode` disables TLS features for faster dev builds.
    pub fn generate(
        &self,
        project_name: &str,
        output_dir: &Path,
        gale_dep_override: Option<&str>,
        dev_mode: bool,
    ) -> Result<(), std::io::Error> {
        let merged = self.merge_programs();
        let interner = TypeInterner::new();

        // Build a component-name → filesystem-URL-path map from the
        // discovered routes so the codegen uses filesystem paths (e.g.
        // "/about") instead of deriving from PascalCase names ("/about-page").
        let route_overrides = self.build_route_overrides();

        crate::codegen::generate(
            &merged,
            &interner,
            project_name,
            output_dir,
            gale_dep_override,
            &route_overrides,
            dev_mode,
        )
    }

    /// Build a map from component name to filesystem-derived URL path.
    ///
    /// Walks the discovered routes, finds the `ComponentDecl` name in each
    /// route's parsed page file, and maps it to the route's `url_path`.
    fn build_route_overrides(&self) -> std::collections::HashMap<String, String> {
        use crate::ast::Item;

        // Build a reverse map: canonical file_path → file_id
        let mut path_to_file_id: std::collections::HashMap<PathBuf, u32> =
            std::collections::HashMap::new();
        for file_id in self.sources.keys() {
            if let Some(path) = self.file_table.get_path(*file_id) {
                let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
                path_to_file_id.insert(canonical, *file_id);
            }
        }

        let mut overrides = std::collections::HashMap::new();

        for route in &self.routes {
            let canonical = route
                .page_file
                .canonicalize()
                .unwrap_or_else(|_| route.page_file.clone());
            let file_id = match path_to_file_id.get(&canonical) {
                Some(id) => *id,
                None => continue,
            };

            // Find the ComponentDecl in this file's parsed program
            for (prog_file_id, program) in &self.programs {
                if *prog_file_id != file_id {
                    continue;
                }
                for item in &program.items {
                    let comp_name = match item {
                        Item::ComponentDecl(d) => Some(&d.name),
                        Item::Out(out) => match out.inner.as_ref() {
                            Item::ComponentDecl(d) => Some(&d.name),
                            _ => None,
                        },
                        _ => None,
                    };
                    if let Some(name) = comp_name {
                        overrides.insert(name.to_string(), route.url_path.clone());
                    }
                }
            }
        }

        overrides
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

        crate::tailwind::generate::run_tailwind(&tw_config, app_dir, &safelist, &output_css, minify)
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
