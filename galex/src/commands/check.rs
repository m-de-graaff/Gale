//! `gale check` — type-check .gx files without code generation.
//!
//! Runs the full parse + typecheck pipeline and reports all errors
//! with structured diagnostics. Exits 0 on success, 1 on errors.

use std::path::Path;

use crate::compiler::Compiler;
use crate::router;

/// Run the check command.
///
/// Returns the process exit code (0 = success, 1 = errors found).
pub fn run(app_dir: &Path) -> i32 {
    // Step 1: Discover routes
    let routes = match router::discovery::discover_routes(app_dir) {
        Ok(r) => r,
        Err(errors) => {
            for err in &errors {
                eprintln!("  error: {err}");
            }
            return 1;
        }
    };

    // Step 2: Load and parse all .gx files (dedup shared layouts/guards)
    let mut compiler = Compiler::new();
    for route in &routes {
        let _ = compiler.add_file_dedup(&route.page_file);
        for layout in &route.layouts {
            let _ = compiler.add_file_dedup(layout);
        }
        for guard in &route.guards {
            let _ = compiler.add_file_dedup(guard);
        }
        for mw in &route.middleware {
            let _ = compiler.add_file_dedup(mw);
        }
    }

    let parse_err_count = compiler.parse_all();
    if parse_err_count > 0 {
        eprintln!();
        for err in &compiler.parse_errors {
            eprintln!("  {err}");
        }
        eprintln!();
        eprintln!(
            "  Found {} parse error{}",
            parse_err_count,
            if parse_err_count != 1 { "s" } else { "" }
        );
        return 1;
    }

    // Step 3: Type check
    let type_errors = compiler.check();
    if !type_errors.is_empty() {
        eprintln!();
        for err in &type_errors {
            eprintln!("  {err}");
        }
        eprintln!();
        let file_count = count_unique_files(&type_errors);
        eprintln!(
            "  Found {} error{} in {} file{}",
            type_errors.len(),
            if type_errors.len() != 1 { "s" } else { "" },
            file_count,
            if file_count != 1 { "s" } else { "" },
        );
        return 1;
    }

    eprintln!(
        "  Checked {} file{} — no errors",
        compiler.sources.len(),
        if compiler.sources.len() != 1 { "s" } else { "" }
    );
    0
}

/// Count unique file references in error messages (heuristic).
fn count_unique_files(errors: &[String]) -> usize {
    use std::collections::HashSet;
    let mut files = HashSet::new();
    for err in errors {
        // Extract file path from error messages like "path:line:col: message"
        if let Some(colon_pos) = err.find(':') {
            files.insert(&err[..colon_pos]);
        }
    }
    files.len().max(1)
}
