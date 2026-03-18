//! Package manifest format (`gale-package.toml`).

use std::collections::HashMap;
use std::path::Path;

/// Package manifest — `gale-package.toml` inside each package.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PackageManifest {
    /// `[package]` section — required metadata.
    pub package: PackageInfo,
    /// `[dependencies]` — other Gale packages this depends on.
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
    /// `[rust_dependencies]` — Rust crates needed by adapter packages.
    #[serde(default)]
    pub rust_dependencies: HashMap<String, RustDep>,
}

/// Core package metadata.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PackageInfo {
    /// Namespaced name (e.g. `"ui/button"`, `"db/postgres"`).
    pub name: String,
    /// Semver version (e.g. `"1.0.0"`).
    pub version: String,
    /// Short description.
    #[serde(default)]
    pub description: String,
    /// Author name or handle.
    #[serde(default)]
    pub author: String,
    /// SPDX license identifier.
    #[serde(default)]
    pub license: String,
    /// Minimum compatible Gale compiler version.
    #[serde(default)]
    pub gale_version: String,
    /// Package type.
    #[serde(default)]
    pub package_type: PackageType,
}

/// What kind of package this is.
#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageType {
    /// Component package — `.gx` files with `out ui`.
    #[default]
    Component,
    /// Adapter package — Rust crate + `.gx` bindings (e.g. `db/postgres`).
    Adapter,
    /// Utility package — shared guards, types, pure functions.
    Utility,
}

/// A Rust crate dependency declared by an adapter package.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct RustDep {
    pub version: String,
    #[serde(default)]
    pub features: Vec<String>,
}

impl PackageManifest {
    /// Load a package manifest from a directory.
    pub fn load(pkg_dir: &Path) -> Result<Self, String> {
        let path = pkg_dir.join("gale-package.toml");
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
        toml::from_str(&content).map_err(|e| format!("failed to parse {}: {e}", path.display()))
    }

    /// The on-disk directory name for this package (`ui/button` → `ui_button`).
    pub fn dir_name(&self) -> String {
        self.package.name.replace('/', "_")
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_component_manifest() {
        let toml_str = r#"
[package]
name = "ui/button"
version = "1.0.0"
description = "A polished button component"
author = "gale-team"
license = "MIT"
gale_version = ">=0.1.0"
"#;
        let manifest: PackageManifest = toml::from_str(toml_str).unwrap();
        assert_eq!(manifest.package.name, "ui/button");
        assert_eq!(manifest.package.version, "1.0.0");
        assert!(matches!(
            manifest.package.package_type,
            PackageType::Component
        ));
    }

    #[test]
    fn parse_adapter_manifest() {
        let toml_str = r#"
[package]
name = "db/postgres"
version = "0.5.0"
description = "PostgreSQL adapter"
package_type = "adapter"

[rust_dependencies]
sqlx = { version = "0.8", features = ["postgres", "runtime-tokio"] }

[dependencies]
"#;
        let manifest: PackageManifest = toml::from_str(toml_str).unwrap();
        assert!(matches!(
            manifest.package.package_type,
            PackageType::Adapter
        ));
        assert!(manifest.rust_dependencies.contains_key("sqlx"));
        let sqlx = &manifest.rust_dependencies["sqlx"];
        assert_eq!(sqlx.version, "0.8");
        assert!(sqlx.features.contains(&"postgres".to_string()));
    }

    #[test]
    fn dir_name_replaces_slashes() {
        let manifest = PackageManifest {
            package: PackageInfo {
                name: "ui/button".into(),
                version: "1.0.0".into(),
                description: String::new(),
                author: String::new(),
                license: String::new(),
                gale_version: String::new(),
                package_type: PackageType::Component,
            },
            dependencies: HashMap::new(),
            rust_dependencies: HashMap::new(),
        };
        assert_eq!(manifest.dir_name(), "ui_button");
    }
}
