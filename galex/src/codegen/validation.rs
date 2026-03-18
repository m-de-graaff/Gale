//! Build-time validation (GX1800-GX1810).
//!
//! Validates build output, gale.toml configuration, and codegen results.

use crate::errors::{codes, Diagnostic};
use crate::span::Span;
use std::path::Path;

/// Validate build output.
///
/// Checks:
/// - GX1806: No routes found (warning)
/// - GX1803: Cannot write to output directory
pub fn validate_build(output_dir: &Path, routes_count: usize) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // GX1806: No routes found (warning)
    if routes_count == 0 {
        diagnostics.push(Diagnostic::new(&codes::GX1806, Span::dummy()));
    }

    // GX1803: Cannot write to output directory
    if output_dir.exists() {
        if let Ok(metadata) = output_dir.metadata() {
            if metadata.permissions().readonly() {
                diagnostics.push(Diagnostic::with_message(
                    &codes::GX1803,
                    format!(
                        "Cannot write to output directory `{}`",
                        output_dir.display()
                    ),
                    Span::dummy(),
                ));
            }
        }
    }

    diagnostics
}

/// Wrap a gale.toml parse error into a GX1804 diagnostic.
pub fn config_parse_error(error_msg: &str) -> Diagnostic {
    Diagnostic::with_message(
        &codes::GX1804,
        format!("`gale.toml` is invalid: {}", error_msg),
        Span::dummy(),
    )
}

/// Wrap a gale.toml version error into a GX1805 diagnostic.
pub fn config_version_unsupported(version: &str) -> Diagnostic {
    Diagnostic::with_message(
        &codes::GX1805,
        format!(
            "`gale.toml` version `{}` is not supported by this compiler",
            version
        ),
        Span::dummy(),
    )
}

/// Wrap a Tailwind CSS compilation failure into a GX1807 diagnostic.
pub fn tailwind_failure(error_msg: &str) -> Diagnostic {
    Diagnostic::with_message(
        &codes::GX1807,
        format!("Tailwind CSS compilation failed: {}", error_msg),
        Span::dummy(),
    )
}

/// Report a static asset not found error (GX1808).
pub fn static_asset_not_found(asset_path: &str) -> Diagnostic {
    Diagnostic::with_message(
        &codes::GX1808,
        format!("Static asset not found: `{}`", asset_path),
        Span::dummy(),
    )
}

/// Report a slow build warning (GX1809).
pub fn slow_build(file_name: &str, duration_secs: f64) -> Diagnostic {
    Diagnostic::with_message(
        &codes::GX1809,
        format!("`{}` took {:.1}s to compile", file_name, duration_secs),
        Span::dummy(),
    )
}

/// Report a codegen target directory collision (GX1810).
pub fn codegen_collision(route_a: &str, route_b: &str) -> Diagnostic {
    Diagnostic::with_message(
        &codes::GX1810,
        format!(
            "Routes `{}` and `{}` would generate the same output file",
            route_a, route_b
        ),
        Span::dummy(),
    )
}

/// Report a generated Rust code compilation failure (GX1800).
pub fn codegen_compile_error(details: &str) -> Diagnostic {
    Diagnostic::with_message(
        &codes::GX1800,
        format!(
            "Generated Rust code failed to compile — this is a compiler bug: {}",
            details
        ),
        Span::dummy(),
    )
}

/// Report a generated JS exceeding size limit (GX1801).
pub fn js_size_limit(page_name: &str, size_kb: f64) -> Diagnostic {
    Diagnostic::with_message(
        &codes::GX1801,
        format!(
            "Generated JS for page `{}` is {:.0}KB (limit: 50KB)",
            page_name, size_kb
        ),
        Span::dummy(),
    )
}

/// Report a Cargo build failure (GX1802).
pub fn cargo_build_failed(stderr: &str) -> Diagnostic {
    Diagnostic::with_message(
        &codes::GX1802,
        format!("Cargo build failed: {}", stderr),
        Span::dummy(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn no_routes_produces_warning() {
        let diags = validate_build(&PathBuf::from("/tmp/out"), 0);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code.code, 1806);
        assert!(diags[0].is_warning());
    }

    #[test]
    fn has_routes_no_warning() {
        let diags = validate_build(&PathBuf::from("/tmp/out"), 3);
        // No GX1806 warning when routes exist
        assert!(!diags.iter().any(|d| d.code.code == 1806));
    }

    #[test]
    fn config_parse_error_diagnostic() {
        let diag = config_parse_error("missing field 'name'");
        assert_eq!(diag.code.code, 1804);
        assert!(diag.message.contains("missing field"));
    }

    #[test]
    fn tailwind_failure_diagnostic() {
        let diag = tailwind_failure("invalid config");
        assert_eq!(diag.code.code, 1807);
        assert!(diag.message.contains("invalid config"));
    }
}
