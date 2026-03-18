//! Unified galex.toml configuration.
//!
//! Single serde-based config struct replacing the previous fragmented
//! hand-written parsers. Loaded once and shared across all subsystems.

use std::collections::HashMap;
use std::path::Path;

/// Complete galex.toml configuration.
#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct GalexConfig {
    /// `[project]` section.
    pub project: Option<ProjectConfig>,
    /// `[tailwind]` section.
    pub tailwind: Option<TailwindSection>,
    /// `[database]` section.
    pub database: Option<DatabaseConfig>,
    /// `[auth]` section.
    pub auth: Option<AuthConfig>,
    /// `[dependencies]` section — package name → version spec.
    #[serde(default)]
    pub dependencies: HashMap<String, DependencySpec>,
}

/// `[project]` section.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ProjectConfig {
    pub name: String,
}

/// `[tailwind]` section.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct TailwindSection {
    #[serde(default)]
    pub enabled: bool,
    pub primary: Option<String>,
    pub font_sans: Option<String>,
    #[serde(default)]
    pub content: Vec<String>,
    pub input_css: Option<String>,
}

/// `[database]` section.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct DatabaseConfig {
    pub adapter: String,
}

/// `[auth]` section.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct AuthConfig {
    pub strategy: String,
}

/// Dependency version specification.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(untagged)]
pub enum DependencySpec {
    /// Simple version string: `"1.2.0"` or `"^1.0"`.
    Version(String),
    /// Detailed spec: `{ version = "1.0", features = ["full"] }`.
    Detailed {
        version: String,
        #[serde(default)]
        features: Vec<String>,
    },
}

impl DependencySpec {
    /// Get the version string regardless of spec variant.
    pub fn version(&self) -> &str {
        match self {
            DependencySpec::Version(v) => v,
            DependencySpec::Detailed { version, .. } => version,
        }
    }
}

impl GalexConfig {
    /// Load configuration from `galex.toml` in the given directory.
    ///
    /// Returns default config if the file doesn't exist or can't be parsed.
    pub fn load(project_dir: &Path) -> Self {
        let path = project_dir.join("galex.toml");
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };
        toml::from_str(&content).unwrap_or_default()
    }

    /// Save configuration to `galex.toml` in the given directory.
    pub fn save(&self, project_dir: &Path) -> Result<(), std::io::Error> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(project_dir.join("galex.toml"), content)
    }

    /// Check if Tailwind CSS is enabled.
    pub fn tailwind_enabled(&self) -> bool {
        self.tailwind.as_ref().map(|t| t.enabled).unwrap_or(false)
    }

    /// Convert the tailwind section to the legacy TailwindConfig format
    /// used by the existing tailwind codegen.
    pub fn to_tailwind_config(&self) -> crate::tailwind::config::TailwindConfig {
        match &self.tailwind {
            Some(tw) => crate::tailwind::config::TailwindConfig {
                enabled: tw.enabled,
                primary: tw.primary.clone(),
                font_sans: tw.font_sans.clone(),
                content: tw.content.clone(),
                input_css: tw.input_css.as_ref().map(std::path::PathBuf::from),
            },
            None => crate::tailwind::config::TailwindConfig::default(),
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_config() {
        let config: GalexConfig = toml::from_str("").unwrap();
        assert!(config.project.is_none());
        assert!(config.tailwind.is_none());
        assert!(config.dependencies.is_empty());
    }

    #[test]
    fn parse_full_config() {
        let toml_str = r##"
[project]
name = "my-app"

[tailwind]
enabled = true
primary = "#3B82F6"

[database]
adapter = "postgres"

[auth]
strategy = "session"

[dependencies]
db-postgres = "1.0.0"
ui-button = { version = "2.1", features = ["icons"] }
"##;
        let config: GalexConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.project.as_ref().unwrap().name, "my-app");
        assert!(config.tailwind_enabled());
        assert_eq!(config.database.as_ref().unwrap().adapter, "postgres");
        assert_eq!(config.dependencies.len(), 2);
        assert_eq!(config.dependencies["db-postgres"].version(), "1.0.0");
        assert_eq!(config.dependencies["ui-button"].version(), "2.1");
    }

    #[test]
    fn roundtrip_save_load() {
        let dir = std::env::temp_dir().join("gale_config_test");
        std::fs::create_dir_all(&dir).ok();

        let mut config = GalexConfig::default();
        config.project = Some(ProjectConfig {
            name: "test".into(),
        });
        config.dependencies.insert(
            "db-postgres".into(),
            DependencySpec::Version("1.0.0".into()),
        );
        config.save(&dir).unwrap();

        let loaded = GalexConfig::load(&dir);
        assert_eq!(loaded.project.as_ref().unwrap().name, "test");
        assert_eq!(loaded.dependencies.len(), 1);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn dependency_spec_version() {
        let simple = DependencySpec::Version("1.0.0".into());
        assert_eq!(simple.version(), "1.0.0");

        let detailed = DependencySpec::Detailed {
            version: "2.0".into(),
            features: vec!["full".into()],
        };
        assert_eq!(detailed.version(), "2.0");
    }
}
