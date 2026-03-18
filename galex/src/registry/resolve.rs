//! Dependency resolution with semver constraints.
//!
//! Resolves a flat dependency tree from `galex.toml [dependencies]`
//! by fetching package metadata from the registry and following
//! transitive dependencies.

use std::collections::HashMap;

use super::client::RegistryClient;
use super::lockfile::Lockfile;
use super::PackageMeta;
use crate::config::{DependencySpec, GalexConfig};

/// A fully resolved package ready to install.
#[derive(Debug, Clone)]
pub struct ResolvedPackage {
    pub name: String,
    pub version: semver::Version,
    pub checksum: String,
    pub download_url: String,
}

/// Result of dependency resolution.
#[derive(Debug, Clone, Default)]
pub struct ResolvedDeps {
    pub packages: Vec<ResolvedPackage>,
}

/// Errors during resolution.
#[derive(Debug)]
pub enum ResolveError {
    /// A package was not found in the registry.
    NotFound(String),
    /// No version matches the constraint.
    NoMatchingVersion { package: String, constraint: String },
    /// Version conflict — two packages need incompatible versions.
    Conflict {
        package: String,
        required_by: Vec<String>,
    },
    /// Network error during resolution.
    Network(String),
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolveError::NotFound(name) => write!(f, "package not found: {name}"),
            ResolveError::NoMatchingVersion {
                package,
                constraint,
            } => {
                write!(
                    f,
                    "no version of '{package}' matches constraint '{constraint}'"
                )
            }
            ResolveError::Conflict {
                package,
                required_by,
            } => {
                write!(
                    f,
                    "version conflict for '{package}', required by: {}",
                    required_by.join(", ")
                )
            }
            ResolveError::Network(msg) => write!(f, "network error: {msg}"),
        }
    }
}

/// Resolve all dependencies from the project config.
///
/// Uses the lockfile for deterministic resolution — locked versions are
/// preferred if they satisfy the constraint. New/updated packages fetch
/// the latest matching version from the registry.
pub async fn resolve_deps(
    config: &GalexConfig,
    client: &RegistryClient,
    lockfile: &Lockfile,
) -> Result<ResolvedDeps, ResolveError> {
    let mut resolved: HashMap<String, ResolvedPackage> = HashMap::new();
    let mut queue: Vec<(String, String, String)> = Vec::new(); // (name, constraint, required_by)

    // Seed the queue with direct dependencies
    for (name, spec) in &config.dependencies {
        let pkg_name = name.replace('-', "/"); // galex.toml uses hyphens, registry uses slashes
        queue.push((pkg_name, spec.version().to_string(), "project".into()));
    }

    // BFS resolution
    while let Some((name, constraint_str, required_by)) = queue.pop() {
        if resolved.contains_key(&name) {
            // Already resolved — check for conflicts
            let existing = &resolved[&name];
            let req = parse_constraint(&constraint_str)?;
            if !req.matches(&existing.version) {
                return Err(ResolveError::Conflict {
                    package: name,
                    required_by: vec![required_by],
                });
            }
            continue;
        }

        // Check lockfile first
        let locked_name = name.replace('/', "-");
        if let Some(locked) = lockfile.resolve(&locked_name) {
            let locked_ver = semver::Version::parse(&locked.version).map_err(|_| {
                ResolveError::NoMatchingVersion {
                    package: name.clone(),
                    constraint: constraint_str.clone(),
                }
            })?;
            let req = parse_constraint(&constraint_str)?;
            if req.matches(&locked_ver) {
                // Use locked version
                resolved.insert(
                    name.clone(),
                    ResolvedPackage {
                        name: name.clone(),
                        version: locked_ver,
                        checksum: locked.checksum.clone(),
                        download_url: String::new(), // Will be fetched if needed
                    },
                );
                continue;
            }
        }

        // Fetch from registry
        let meta = client.fetch_meta(&name).await.map_err(|e| {
            if matches!(e, super::RegistryError::NotFound(_)) {
                ResolveError::NotFound(name.clone())
            } else {
                ResolveError::Network(e.to_string())
            }
        })?;

        let version =
            semver::Version::parse(&meta.version).map_err(|_| ResolveError::NoMatchingVersion {
                package: name.clone(),
                constraint: constraint_str.clone(),
            })?;

        let req = parse_constraint(&constraint_str)?;
        if !req.matches(&version) {
            return Err(ResolveError::NoMatchingVersion {
                package: name.clone(),
                constraint: constraint_str,
            });
        }

        resolved.insert(
            name.clone(),
            ResolvedPackage {
                name: name.clone(),
                version,
                checksum: meta.checksum.clone(),
                download_url: meta.download_url.clone(),
            },
        );

        // Queue transitive dependencies
        for dep in &meta.dependencies {
            queue.push((dep.clone(), "*".into(), name.clone()));
        }
    }

    Ok(ResolvedDeps {
        packages: resolved.into_values().collect(),
    })
}

/// Parse a version constraint string.
fn parse_constraint(spec: &str) -> Result<semver::VersionReq, ResolveError> {
    if spec == "*" {
        return Ok(semver::VersionReq::STAR);
    }
    semver::VersionReq::parse(spec).map_err(|_| ResolveError::NoMatchingVersion {
        package: String::new(),
        constraint: spec.to_string(),
    })
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_version_constraints() {
        assert!(parse_constraint("1.0.0").is_ok());
        assert!(parse_constraint("^1.0").is_ok());
        assert!(parse_constraint(">=1.0.0, <2.0.0").is_ok());
        assert!(parse_constraint("*").is_ok());
    }

    #[test]
    fn star_matches_anything() {
        let req = parse_constraint("*").unwrap();
        assert!(req.matches(&semver::Version::new(1, 0, 0)));
        assert!(req.matches(&semver::Version::new(99, 99, 99)));
    }

    #[test]
    fn caret_constraint() {
        let req = parse_constraint("^1.0").unwrap();
        assert!(req.matches(&semver::Version::new(1, 0, 0)));
        assert!(req.matches(&semver::Version::new(1, 5, 0)));
        assert!(!req.matches(&semver::Version::new(2, 0, 0)));
    }
}
