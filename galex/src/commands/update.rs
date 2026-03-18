//! `gale update` — update packages to latest matching versions.

use std::path::Path;

use crate::config::GalexConfig;
use crate::registry::client::RegistryClient;
use crate::registry::lockfile::{LockedPackage, Lockfile};
use crate::registry::tarball;

/// Run the `gale update` command.
pub fn run(package: Option<&str>) -> i32 {
    let project_dir = Path::new(".");
    let config = GalexConfig::load(project_dir);
    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");

    if config.dependencies.is_empty() {
        eprintln!("  No dependencies to update");
        return 0;
    }

    rt.block_on(async {
        let client = RegistryClient::new();
        let lock_path = project_dir.join("gale.lock");
        let mut lockfile = Lockfile::load(&lock_path);
        let modules_dir = project_dir.join("gale_modules");
        let mut updated = 0;

        let deps_to_update: Vec<(String, String)> = config
            .dependencies
            .iter()
            .filter(|(name, _)| {
                package
                    .map(|p| name.as_str() == p.replace('/', "-"))
                    .unwrap_or(true)
            })
            .map(|(name, spec)| (name.clone(), spec.version().to_string()))
            .collect();

        for (dep_key, _version_spec) in &deps_to_update {
            let pkg_name = dep_key.replace('-', "/");
            eprintln!("  Checking {pkg_name}...");

            // Fetch latest version from registry
            let meta = match client.fetch_meta(&pkg_name).await {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("    warning: {e}");
                    continue;
                }
            };

            // Check if update is available
            let current = lockfile.resolve(dep_key).map(|p| p.version.as_str());
            if current == Some(&meta.version) {
                eprintln!("    Already at latest: v{}", meta.version);
                continue;
            }

            // Download and extract
            std::fs::create_dir_all(&modules_dir).ok();
            match client.download(&meta, &modules_dir).await {
                Ok(bytes) => {
                    let pkg_dir = modules_dir.join(meta.name.replace('/', "_"));
                    let _ = std::fs::remove_dir_all(&pkg_dir);
                    if let Err(e) = tarball::extract(&bytes, &pkg_dir) {
                        eprintln!("    error extracting: {e}");
                        continue;
                    }
                }
                Err(e) => {
                    eprintln!("    error downloading: {e}");
                    continue;
                }
            }

            // Update lockfile
            lockfile.add(LockedPackage {
                name: dep_key.clone(),
                version: meta.version.clone(),
                checksum: meta.checksum.clone(),
            });

            eprintln!("    Updated {} → v{}", pkg_name, meta.version);
            updated += 1;
        }

        if let Err(e) = lockfile.save(&lock_path) {
            eprintln!("  warning: failed to update gale.lock: {e}");
        }

        if updated == 0 {
            eprintln!("  All packages are up to date");
        } else {
            eprintln!(
                "  Updated {} package{}",
                updated,
                if updated != 1 { "s" } else { "" }
            );
        }

        0
    })
}
