//! `gale lint` — run static analysis on .gx files.

use std::path::Path;

use crate::compiler::Compiler;
use crate::lint;
use crate::router;

/// Run the `gale lint` command.
pub fn run(app_dir: &Path) -> i32 {
    // Discover + parse
    let routes = match router::discovery::discover_routes(app_dir) {
        Ok(r) => r,
        Err(errors) => {
            for err in &errors {
                eprintln!("  error: {err}");
            }
            return 1;
        }
    };

    let mut compiler = Compiler::new();
    for route in &routes {
        let _ = compiler.add_file(&route.page_file);
        for layout in &route.layouts {
            let _ = compiler.add_file(layout);
        }
    }

    let parse_err_count = compiler.parse_all();
    if parse_err_count > 0 {
        eprintln!("  {} parse error(s) — fix before linting", parse_err_count);
        return 1;
    }

    // Merge and lint
    let merged = compiler.merge_programs();
    let warnings = lint::lint_program(&merged);

    if warnings.is_empty() {
        eprintln!("  No lint warnings");
        return 0;
    }

    for w in &warnings {
        eprintln!("  {w}");
    }
    eprintln!();
    eprintln!(
        "  {} warning{}",
        warnings.len(),
        if warnings.len() != 1 { "s" } else { "" }
    );
    0 // Warnings don't cause non-zero exit (unlike errors)
}
