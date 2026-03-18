//! `gale test` — discover and run test blocks from .gx files.

use std::path::Path;

use crate::ast::{Item, TestDecl};
use crate::compiler::Compiler;
use crate::router;

/// Run the `gale test` command.
pub fn run(app_dir: &Path, filter: Option<&str>) -> i32 {
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
        for guard in &route.guards {
            let _ = compiler.add_file(guard);
        }
    }

    let parse_err_count = compiler.parse_all();
    if parse_err_count > 0 {
        eprintln!("  {} parse error(s)", parse_err_count);
        return 1;
    }

    // Type check
    let type_errors = compiler.check();
    if !type_errors.is_empty() {
        eprintln!("  {} type error(s)", type_errors.len());
        return 1;
    }

    // Extract test declarations
    let merged = compiler.merge_programs();
    let tests: Vec<&TestDecl> = merged
        .items
        .iter()
        .filter_map(|item| {
            if let Item::TestDecl(t) = item {
                Some(t)
            } else {
                None
            }
        })
        .collect();

    // Apply filter
    let filtered: Vec<&&TestDecl> = if let Some(f) = filter {
        tests.iter().filter(|t| t.name.contains(f)).collect()
    } else {
        tests.iter().collect()
    };

    if filtered.is_empty() {
        if filter.is_some() {
            eprintln!("  No tests matching filter");
        } else {
            eprintln!("  No test blocks found");
        }
        return 0;
    }

    eprintln!(
        "  Found {} test{}",
        filtered.len(),
        if filtered.len() != 1 { "s" } else { "" }
    );

    // Generate and run tests
    super::test_runner::run_tests(&filtered, &compiler)
}
