//! `gale migrate` — upgrade an existing project to the current GaleX version.
//!
//! Deletes `.gale_dev/` and re-runs the full build pipeline with the
//! latest codegen and gale library tag.  Does NOT touch user source files
//! (`app/`, `styles/`, `public/`, `package.json`, etc.).

use std::path::Path;

use crate::compiler::Compiler;
use crate::router;

/// Run the `gale migrate` command.
pub fn run() -> i32 {
    let project_dir = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("  error: cannot determine current directory: {e}");
            return 1;
        }
    };

    let app_dir = project_dir.join("app");
    let output_dir = project_dir.join(".gale_dev");

    if !app_dir.is_dir() {
        eprintln!("  error: no app/ directory found — are you in a Gale project?");
        return 1;
    }

    let version = env!("CARGO_PKG_VERSION");
    eprintln!("  Migrating to v{version}...");

    // Delete old build output to force fresh codegen
    if output_dir.exists() {
        if let Err(e) = std::fs::remove_dir_all(&output_dir) {
            eprintln!("  warning: failed to clean .gale_dev/: {e}");
        }
    }

    // Derive project name from directory
    let project_name = project_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("gale_app")
        .to_string();

    match rebuild_project(&app_dir, &output_dir, &project_name) {
        Ok(()) => {
            eprintln!("  Migration complete!");
            eprintln!();
            eprintln!("  Run `gale dev` to start the dev server.");
            0
        }
        Err(e) => {
            eprintln!("  error: migration failed: {e}");
            1
        }
    }
}

fn rebuild_project(
    app_dir: &Path,
    output_dir: &Path,
    project_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Route discovery
    let routes = router::discovery::discover_routes(app_dir).map_err(|errs| {
        let msg = errs
            .iter()
            .map(|e| e.message.clone())
            .collect::<Vec<_>>()
            .join("; ");
        msg
    })?;

    // Parse + check
    let mut compiler = Compiler::new();
    for route in &routes {
        let files = std::iter::once(&route.page_file)
            .chain(route.layouts.iter())
            .chain(route.guards.iter())
            .chain(route.middleware.iter());
        for path in files {
            let _ = compiler.add_file_dedup(path);
        }
    }
    let parse_errors = compiler.parse_all();
    if parse_errors > 0 {
        return Err("parse errors found — fix them before migrating".into());
    }
    let type_errors = compiler.check();
    if !type_errors.is_empty() {
        return Err(format!(
            "{} type error(s) found — fix them before migrating",
            type_errors.len()
        )
        .into());
    }

    // Codegen
    compiler.set_routes(routes);
    compiler.generate(
        &format!("{project_name}_dev_app"),
        output_dir,
        None,
        true, // dev mode
    )?;

    // CSS generation (non-fatal)
    let project_dir = app_dir.parent().unwrap_or(Path::new("."));
    if let Err(e) = compiler.generate_css(project_dir, app_dir, output_dir, true) {
        eprintln!("  warning: CSS generation failed: {e}");
    }

    // Cargo build
    eprintln!("  Building...");
    let status = std::process::Command::new("cargo")
        .arg("build")
        .current_dir(output_dir)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()?;

    if !status.success() {
        return Err("cargo build failed".into());
    }

    Ok(())
}
