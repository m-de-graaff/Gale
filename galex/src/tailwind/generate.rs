//! Pure-Rust Tailwind CSS generation via the `tailwind_css` crate.
//!
//! No Node.js, no npm, no `node_modules`.  Class names extracted from the
//! GaleX AST are fed to `TailwindBuilder::trace()` and compiled to CSS
//! in-process.

use std::path::Path;

use tailwind_css::TailwindBuilder;

use super::config::TailwindConfig;

/// Errors that can occur during Tailwind CSS generation.
#[derive(Debug)]
pub enum TailwindError {
    /// The Rust Tailwind compiler returned an error.
    BuildFailed(String),
    /// I/O error (file read/write).
    IoError(std::io::Error),
}

impl std::fmt::Display for TailwindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TailwindError::BuildFailed(msg) => write!(f, "Tailwind CSS build failed: {msg}"),
            TailwindError::IoError(e) => write!(f, "I/O error during CSS generation: {e}"),
        }
    }
}

impl From<std::io::Error> for TailwindError {
    fn from(e: std::io::Error) -> Self {
        TailwindError::IoError(e)
    }
}

/// Generate a Tailwind CSS bundle from extracted class names.
///
/// Uses the pure-Rust `tailwind_css` crate — no Node.js required.
///
/// # Arguments
///
/// * `tw_config` — Parsed Tailwind configuration from `galex.toml`
/// * `_app_dir` — Path to the `app/` source directory (unused — classes come from AST)
/// * `safelist` — Class names extracted from the GaleX AST
/// * `output_css` — Where to write the generated CSS file
/// * `_minify` — Reserved for future use (the Rust crate produces compact output)
pub fn run_tailwind(
    _tw_config: &TailwindConfig,
    _app_dir: &Path,
    safelist: &[String],
    output_css: &Path,
    _minify: bool,
) -> Result<(), TailwindError> {
    let mut builder = TailwindBuilder::default();

    // Trace all classes extracted from the GaleX AST.
    // Each class string may contain multiple space-separated utilities
    // (e.g. "flex items-center gap-4"), so split on whitespace.
    // The Rust crate may panic on unsupported utilities (e.g. `bg-black`),
    // so we catch panics and skip those classes gracefully.
    for class_group in safelist {
        for class in class_group.split_whitespace() {
            let class_owned = class.to_string();
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                builder.trace(&class_owned);
            }));
        }
    }

    // Bundle the traced classes into a CSS string.
    let mut css = builder.bundle();

    // Append fallback utilities that the Rust crate doesn't support.
    // These are standard Tailwind utilities that are commonly used but
    // missing from the tailwind_css crate's implementation.
    css.push_str(&generate_fallback_utilities(safelist));

    // Append custom color utilities from config.
    if let Some(ref primary) = _tw_config.primary {
        css.push_str(&generate_color_utilities("primary", primary));
    }

    // Ensure output directory exists.
    if let Some(parent) = output_css.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(output_css, css)?;
    Ok(())
}

/// Generate fallback CSS for utilities the Rust crate doesn't support.
///
/// Scans the safelist for known missing utilities (e.g. `bg-black`,
/// `text-white`, `cursor-pointer`) and emits hand-written CSS rules.
fn generate_fallback_utilities(safelist: &[String]) -> String {
    let mut css = String::new();
    let mut seen = std::collections::HashSet::new();

    // Collect all individual class names
    let classes: Vec<&str> = safelist.iter().flat_map(|g| g.split_whitespace()).collect();

    // Map of class name → CSS rule (only commonly used missing utilities)
    let fallbacks: &[(&str, &str)] = &[
        // Colors: black/white
        ("bg-black", ".bg-black{background-color:#000}"),
        ("bg-white", ".bg-white{background-color:#fff}"),
        ("text-black", ".text-black{color:#000}"),
        ("text-white", ".text-white{color:#fff}"),
        ("border-black", ".border-black{border-color:#000}"),
        ("border-white", ".border-white{border-color:#fff}"),
        // Transparency
        (
            "bg-transparent",
            ".bg-transparent{background-color:transparent}",
        ),
        // Cursor
        ("cursor-pointer", ".cursor-pointer{cursor:pointer}"),
        ("cursor-default", ".cursor-default{cursor:default}"),
        (
            "cursor-not-allowed",
            ".cursor-not-allowed{cursor:not-allowed}",
        ),
        // Common missing utilities
        (
            "tabular-nums",
            ".tabular-nums{font-variant-numeric:tabular-nums}",
        ),
        ("select-none", ".select-none{user-select:none}"),
        ("select-all", ".select-all{user-select:all}"),
        (
            "truncate",
            ".truncate{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}",
        ),
        ("break-all", ".break-all{word-break:break-all}"),
        ("break-words", ".break-words{overflow-wrap:break-word}"),
        (
            "antialiased",
            ".antialiased{-webkit-font-smoothing:antialiased;-moz-osx-font-smoothing:grayscale}",
        ),
        (
            "outline-none",
            ".outline-none{outline:2px solid transparent;outline-offset:2px}",
        ),
        // Overflow
        ("overflow-y-auto", ".overflow-y-auto{overflow-y:auto}"),
        ("overflow-x-auto", ".overflow-x-auto{overflow-x:auto}"),
        ("overflow-hidden", ".overflow-hidden{overflow:hidden}"),
        ("overflow-auto", ".overflow-auto{overflow:auto}"),
    ];

    for (name, rule) in fallbacks {
        if classes.contains(name) && seen.insert(*name) {
            css.push('\n');
            css.push_str(rule);
        }
    }

    css
}

/// Generate CSS utilities for a custom color name.
///
/// Given `name = "primary"` and `value = "#3B82F6"`, generates:
/// ```css
/// .bg-primary { background-color: #3B82F6; }
/// .text-primary { color: #3B82F6; }
/// .border-primary { border-color: #3B82F6; }
/// ```
fn generate_color_utilities(name: &str, value: &str) -> String {
    format!(
        "\n.bg-{name}{{background-color:{value}}}\
         \n.text-{name}{{color:{value}}}\
         \n.border-{name}{{border-color:{value}}}\
         \n.ring-{name}{{--tw-ring-color:{value}}}\n"
    )
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tailwind_error_display() {
        let err = TailwindError::BuildFailed("exit 1".into());
        let msg = format!("{err}");
        assert!(msg.contains("build failed"));
    }

    #[test]
    fn generate_basic_css() {
        let config = TailwindConfig::default();
        let dir = std::env::temp_dir().join("gale_test_tw_gen");
        std::fs::create_dir_all(&dir).ok();
        let output = dir.join("styles.css");

        let classes = vec![
            "flex items-center".to_string(),
            "bg-zinc-950 text-zinc-50".to_string(),
            "p-4 rounded-lg".to_string(),
            "bg-black".to_string(), // unsupported by crate — should be skipped gracefully
        ];

        let result = run_tailwind(
            &config,
            std::path::Path::new("app"),
            &classes,
            &output,
            false,
        );
        assert!(result.is_ok(), "generation should succeed");
        assert!(output.exists(), "CSS file should be created");

        let css = std::fs::read_to_string(&output).unwrap();
        assert!(!css.is_empty(), "CSS should not be empty");

        std::fs::remove_dir_all(&dir).ok();
    }
}
