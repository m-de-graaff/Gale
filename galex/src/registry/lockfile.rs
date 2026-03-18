//! `gale.lock` file reader/writer.
//!
//! Tracks exact versions and checksums of installed packages
//! for reproducible builds.

use std::path::Path;

/// A lock file tracking installed package versions.
#[derive(Debug, Clone, Default)]
pub struct Lockfile {
    /// Locked packages.
    pub packages: Vec<LockedPackage>,
}

/// A single locked package entry.
#[derive(Debug, Clone)]
pub struct LockedPackage {
    /// Package name (e.g. "db/postgres").
    pub name: String,
    /// Exact version (e.g. "1.2.0").
    pub version: String,
    /// SHA-256 checksum of the installed tarball.
    pub checksum: String,
}

impl Lockfile {
    /// Load a lock file from disk. Returns empty lockfile if file doesn't exist.
    pub fn load(path: &Path) -> Self {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };
        parse_lockfile(&content)
    }

    /// Save the lock file to disk.
    pub fn save(&self, path: &Path) -> Result<(), std::io::Error> {
        let mut out = String::from("# gale.lock — auto-generated, do not edit\n\n");
        for pkg in &self.packages {
            out.push_str(&format!("[[package]]\n"));
            out.push_str(&format!("name = \"{}\"\n", pkg.name));
            out.push_str(&format!("version = \"{}\"\n", pkg.version));
            out.push_str(&format!("checksum = \"{}\"\n\n", pkg.checksum));
        }
        std::fs::write(path, out)
    }

    /// Add or update a package in the lock file.
    pub fn add(&mut self, pkg: LockedPackage) {
        // Remove existing entry with same name
        self.packages.retain(|p| p.name != pkg.name);
        self.packages.push(pkg);
        self.packages.sort_by(|a, b| a.name.cmp(&b.name));
    }

    /// Resolve a package by name.
    pub fn resolve(&self, name: &str) -> Option<&LockedPackage> {
        self.packages.iter().find(|p| p.name == name)
    }
}

/// Parse a gale.lock file.
fn parse_lockfile(content: &str) -> Lockfile {
    let mut packages = Vec::new();
    let mut current_name = None;
    let mut current_version = None;
    let mut current_checksum = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed == "[[package]]" {
            // Flush previous package
            if let (Some(name), Some(version), Some(checksum)) = (
                current_name.take(),
                current_version.take(),
                current_checksum.take(),
            ) {
                packages.push(LockedPackage {
                    name,
                    version,
                    checksum,
                });
            }
            continue;
        }
        if let Some((key, value)) = trimmed.split_once('=') {
            let key = key.trim();
            let value = value.trim().trim_matches('"');
            match key {
                "name" => current_name = Some(value.to_string()),
                "version" => current_version = Some(value.to_string()),
                "checksum" => current_checksum = Some(value.to_string()),
                _ => {}
            }
        }
    }
    // Flush last package
    if let (Some(name), Some(version), Some(checksum)) =
        (current_name, current_version, current_checksum)
    {
        packages.push(LockedPackage {
            name,
            version,
            checksum,
        });
    }

    Lockfile { packages }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_lockfile() {
        let lf = parse_lockfile("");
        assert!(lf.packages.is_empty());
    }

    #[test]
    fn parse_single_package() {
        let content = r#"
[[package]]
name = "db/postgres"
version = "1.0.0"
checksum = "abc123"
"#;
        let lf = parse_lockfile(content);
        assert_eq!(lf.packages.len(), 1);
        assert_eq!(lf.packages[0].name, "db/postgres");
        assert_eq!(lf.packages[0].version, "1.0.0");
    }

    #[test]
    fn parse_multiple_packages() {
        let content = r#"
[[package]]
name = "auth/session"
version = "0.5.0"
checksum = "def456"

[[package]]
name = "db/postgres"
version = "1.0.0"
checksum = "abc123"
"#;
        let lf = parse_lockfile(content);
        assert_eq!(lf.packages.len(), 2);
    }

    #[test]
    fn add_and_resolve() {
        let mut lf = Lockfile::default();
        lf.add(LockedPackage {
            name: "db/postgres".into(),
            version: "1.0.0".into(),
            checksum: "abc".into(),
        });
        assert!(lf.resolve("db/postgres").is_some());
        assert!(lf.resolve("db/sqlite").is_none());
    }

    #[test]
    fn add_replaces_existing() {
        let mut lf = Lockfile::default();
        lf.add(LockedPackage {
            name: "db/postgres".into(),
            version: "1.0.0".into(),
            checksum: "old".into(),
        });
        lf.add(LockedPackage {
            name: "db/postgres".into(),
            version: "1.1.0".into(),
            checksum: "new".into(),
        });
        assert_eq!(lf.packages.len(), 1);
        assert_eq!(lf.resolve("db/postgres").unwrap().version, "1.1.0");
    }

    #[test]
    fn roundtrip_save_load() {
        let dir = std::env::temp_dir().join("gale_lockfile_test");
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("gale.lock");

        let mut lf = Lockfile::default();
        lf.add(LockedPackage {
            name: "db/postgres".into(),
            version: "1.0.0".into(),
            checksum: "abc123".into(),
        });
        lf.save(&path).unwrap();

        let loaded = Lockfile::load(&path);
        assert_eq!(loaded.packages.len(), 1);
        assert_eq!(loaded.packages[0].name, "db/postgres");
        std::fs::remove_dir_all(&dir).ok();
    }
}
