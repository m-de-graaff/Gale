//! `gale fmt` — format .gx source files.

use std::path::Path;
use walkdir::WalkDir;

/// Run the `gale fmt` command.
///
/// If `check_only` is true, reports unformatted files and exits 1 without writing.
pub fn run(app_dir: &Path, check_only: bool) -> i32 {
    // The formatter is currently disabled due to correctness issues.
    // It will silently alter program semantics in ways that break code:
    //   - Parenthesized expressions lose grouping: (a + b) * c  →  a + b * c
    //   - All comments (// and /* */) are stripped
    //   - Template text whitespace is trimmed
    let _ = (app_dir, check_only);
    eprintln!();
    eprintln!("  error: gale fmt is temporarily disabled");
    eprintln!();
    eprintln!("  The formatter has known issues that silently alter program semantics:");
    eprintln!("    - Parenthesized expressions lose their grouping");
    eprintln!("      (a + b) * c  becomes  a + b * c");
    eprintln!("    - All comments are removed");
    eprintln!();
    eprintln!("  This will be fixed in a future release.");
    eprintln!();
    return 1;

    #[allow(unreachable_code)]
    let files = find_gx_files(app_dir);
    if files.is_empty() {
        eprintln!("  No .gx files found in {}", app_dir.display());
        return 0;
    }

    let mut unformatted = 0;
    let mut formatted_count = 0;
    let mut error_count = 0;

    for file in &files {
        let source = match std::fs::read_to_string(file) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("  error reading {}: {e}", file.display());
                error_count += 1;
                continue;
            }
        };

        match crate::fmt::format_source(&source, 0) {
            Ok(formatted) => {
                if source != formatted {
                    if check_only {
                        eprintln!("  {}: not formatted", file.display());
                        unformatted += 1;
                    } else {
                        if let Err(e) = std::fs::write(file, &formatted) {
                            eprintln!("  error writing {}: {e}", file.display());
                            error_count += 1;
                        } else {
                            eprintln!("  Formatted {}", file.display());
                            formatted_count += 1;
                        }
                    }
                }
            }
            Err(errors) => {
                eprintln!("  {} (skipped — parse errors):", file.display());
                for err in errors.iter().take(3) {
                    eprintln!("    {err}");
                }
                error_count += 1;
            }
        }
    }

    if check_only {
        if unformatted > 0 {
            eprintln!(
                "  {} file{} not formatted",
                unformatted,
                if unformatted != 1 { "s" } else { "" }
            );
            return 1;
        }
        eprintln!("  All files formatted correctly");
    } else if formatted_count > 0 {
        eprintln!(
            "  Formatted {} file{}",
            formatted_count,
            if formatted_count != 1 { "s" } else { "" }
        );
    } else if error_count == 0 {
        eprintln!("  All files already formatted");
    }

    if error_count > 0 {
        1
    } else {
        0
    }
}

/// Find all .gx files under a directory.
pub fn find_gx_files(dir: &Path) -> Vec<std::path::PathBuf> {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file()
                && e.path().extension().and_then(|ext| ext.to_str()) == Some("gx")
        })
        .map(|e| e.path().to_path_buf())
        .collect()
}
