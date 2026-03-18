//! Tailwind configuration loading and `tailwind.config.js` generation.
//!
//! Loads the `[tailwind]` section from `galex.toml` and generates a
//! `tailwind.config.js` file that the Tailwind CLI can consume.

use std::path::{Path, PathBuf};

/// Tailwind configuration extracted from `galex.toml`.
#[derive(Debug, Clone)]
pub struct TailwindConfig {
    /// Whether Tailwind is enabled. Defaults to `true` if `[tailwind]` section exists.
    pub enabled: bool,
    /// Custom primary color (e.g. `"#3B82F6"`).
    pub primary: Option<String>,
    /// Custom sans-serif font family.
    pub font_sans: Option<String>,
    /// Additional content paths to scan (beyond `app/**/*.gx`).
    pub content: Vec<String>,
    /// Path to custom input CSS file (for `@apply`, etc.).
    pub input_css: Option<PathBuf>,
}

impl Default for TailwindConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            primary: None,
            font_sans: None,
            content: Vec::new(),
            input_css: None,
        }
    }
}

/// Load Tailwind configuration from `galex.toml` in the given directory.
///
/// If the file doesn't exist or has no `[tailwind]` section, returns
/// a default (disabled) config.
pub fn load_config(project_dir: &Path) -> TailwindConfig {
    let config_path = project_dir.join("galex.toml");
    if !config_path.is_file() {
        return TailwindConfig::default();
    }

    let content = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(_) => return TailwindConfig::default(),
    };

    parse_tailwind_section(&content)
}

/// Parse the `[tailwind]` section from TOML content.
///
/// This is a minimal hand-parser for the flat key-value structure we need,
/// avoiding a full TOML crate dependency.
fn parse_tailwind_section(content: &str) -> TailwindConfig {
    let mut config = TailwindConfig::default();
    let mut in_tailwind = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip comments and empty lines
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Section headers
        if trimmed.starts_with('[') {
            in_tailwind = trimmed == "[tailwind]";
            continue;
        }

        if !in_tailwind {
            continue;
        }

        // Parse key = value
        if let Some((key, value)) = trimmed.split_once('=') {
            let key = key.trim();
            let value = value.trim();

            match key {
                "enabled" => config.enabled = value == "true",
                "primary" => config.primary = Some(unquote(value)),
                "font_sans" => config.font_sans = Some(unquote(value)),
                "input_css" => config.input_css = Some(PathBuf::from(unquote(value))),
                "content" => {
                    // Parse array: ["path1", "path2"]
                    config.content = parse_string_array(value);
                }
                _ => {} // ignore unknown keys
            }

            if key != "enabled" && config.primary.is_some() && !config.enabled {
                // If any tailwind setting is provided, enable it
                config.enabled = true;
            }
        }
    }

    config
}

/// Remove surrounding quotes from a TOML string value.
///
/// Only strips a matching pair of outermost quotes (not internal ones).
fn unquote(s: &str) -> String {
    let s = s.trim();
    if s.len() >= 2 {
        if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
            return s[1..s.len() - 1].to_string();
        }
    }
    s.to_string()
}

/// Parse a TOML-style string array: `["a", "b", "c"]`.
fn parse_string_array(s: &str) -> Vec<String> {
    let inner = s.trim().trim_start_matches('[').trim_end_matches(']');
    inner
        .split(',')
        .map(|item| unquote(item.trim()))
        .filter(|item| !item.is_empty())
        .collect()
}

/// Generate a `tailwind.config.js` file at the given path.
///
/// Returns the path to the generated config file.
pub fn generate_tailwind_config(
    config: &TailwindConfig,
    app_dir: &Path,
    safelist: &[String],
    output_dir: &Path,
) -> PathBuf {
    let config_path = output_dir.join("tailwind.config.js");

    let app_glob = format!("{}/**/*.gx", app_dir.to_string_lossy().replace('\\', "/"));
    let mut content_paths = vec![format!("'{app_glob}'")];
    for extra in &config.content {
        content_paths.push(format!("'{extra}'"));
    }
    let content_js = content_paths.join(",\n    ");

    // Safelist
    let safelist_js = safelist
        .iter()
        .map(|c| format!("    '{c}'"))
        .collect::<Vec<_>>()
        .join(",\n");

    // Theme extensions
    let mut theme_extends = Vec::new();
    if let Some(ref primary) = config.primary {
        theme_extends.push(format!("      colors: {{ primary: '{}' }}", primary));
    }
    if let Some(ref font) = config.font_sans {
        theme_extends.push(format!("      fontFamily: {{ sans: [{}] }}", font));
    }
    let theme_section = if theme_extends.is_empty() {
        String::new()
    } else {
        format!(
            "  theme: {{\n    extend: {{\n{}\n    }},\n  }},\n",
            theme_extends.join(",\n")
        )
    };

    let config_content = format!(
        r#"/** @type {{import('tailwindcss').Config}} */
module.exports = {{
  content: [
    {content_js}
  ],
  safelist: [
{safelist_js}
  ],
{theme_section}  plugins: [],
}};
"#
    );

    std::fs::write(&config_path, config_content).ok();
    config_path
}

/// Generate the default Tailwind input CSS.
///
/// If the user provided a custom `input_css` in config, that file is used
/// instead (the caller handles this).
pub fn default_input_css() -> String {
    r#"@tailwind base;
@tailwind components;
@tailwind utilities;
"#
    .to_string()
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_config() {
        let config = parse_tailwind_section("");
        assert!(!config.enabled);
    }

    #[test]
    fn parse_basic_tailwind_section() {
        let toml = r##"
[server]
port = 8080

[tailwind]
enabled = true
primary = "#3B82F6"
font_sans = "'Inter', sans-serif"
content = ["lib/**/*.gx", "components/**/*.gx"]
"##;
        let config = parse_tailwind_section(toml);
        assert!(config.enabled);
        assert_eq!(config.primary.as_deref(), Some("#3B82F6"));
        assert_eq!(config.font_sans.as_deref(), Some("'Inter', sans-serif"));
        assert_eq!(config.content.len(), 2);
        assert_eq!(config.content[0], "lib/**/*.gx");
    }

    #[test]
    fn parse_with_input_css() {
        let toml = r#"
[tailwind]
enabled = true
input_css = "styles/global.css"
"#;
        let config = parse_tailwind_section(toml);
        assert!(config.enabled);
        assert_eq!(
            config.input_css.as_deref(),
            Some(Path::new("styles/global.css"))
        );
    }

    #[test]
    fn parse_ignores_other_sections() {
        let toml = r##"
[server]
port = 8080
primary = "not-a-color"

[tailwind]
primary = "#FF0000"
"##;
        let config = parse_tailwind_section(toml);
        assert_eq!(config.primary.as_deref(), Some("#FF0000"));
    }

    #[test]
    fn generate_config_file() {
        let config = TailwindConfig {
            enabled: true,
            primary: Some("#3B82F6".into()),
            font_sans: None,
            content: vec!["lib/**/*.gx".into()],
            input_css: None,
        };
        let dir = std::env::temp_dir().join("gale_test_tw");
        std::fs::create_dir_all(&dir).ok();
        let path = generate_tailwind_config(
            &config,
            Path::new("app"),
            &["bg-blue-500".into(), "text-white".into()],
            &dir,
        );
        assert!(path.exists(), "config file should be created");
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("content:"), "has content: {content}");
        assert!(content.contains("app/**/*.gx"), "has app glob: {content}");
        assert!(
            content.contains("lib/**/*.gx"),
            "has extra content: {content}"
        );
        assert!(content.contains("bg-blue-500"), "has safelist: {content}");
        assert!(content.contains("primary"), "has theme: {content}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn default_css_has_directives() {
        let css = default_input_css();
        assert!(css.contains("@tailwind base"));
        assert!(css.contains("@tailwind components"));
        assert!(css.contains("@tailwind utilities"));
    }

    #[test]
    fn unquote_handles_both_quote_types() {
        assert_eq!(unquote("\"hello\""), "hello");
        assert_eq!(unquote("'world'"), "world");
        assert_eq!(unquote("bare"), "bare");
    }

    #[test]
    fn parse_string_array_works() {
        let result = parse_string_array("[\"a\", \"b\", \"c\"]");
        assert_eq!(result, vec!["a", "b", "c"]);
    }
}
