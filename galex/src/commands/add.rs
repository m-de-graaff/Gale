//! `gale add` — add a package from the registry.

use std::path::Path;

use crate::config::{DependencySpec, GalexConfig};
use crate::registry::client::RegistryClient;
use crate::registry::lockfile::{Lockfile, LockedPackage};
use crate::registry::tarball;

/// Run the `gale add` command.
pub fn run(package: &str) -> i32 {
    let project_dir = Path::new(".");
    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");

    rt.block_on(async {
        let client = RegistryClient::new();

        // 1. Fetch package metadata
        eprintln!("  Fetching package '{package}'...");
        let meta = match client.fetch_meta(package).await {
            Ok(m) => m,
            Err(e) => {
                eprintln!("  error: {e}");
                return 1;
            }
        };
        eprintln!("  Found {} v{}", meta.name, meta.version);

        // 2. Download tarball
        let modules_dir = project_dir.join("gale_modules");
        std::fs::create_dir_all(&modules_dir).ok();
        let tarball_bytes = match client.download(&meta, &modules_dir).await {
            Ok(bytes) => bytes,
            Err(e) => {
                eprintln!("  error: {e}");
                return 1;
            }
        };

        // 3. Extract tarball to gale_modules/{name}/
        let pkg_dir = modules_dir.join(meta.name.replace('/', "_"));
        if let Err(e) = tarball::extract(&tarball_bytes, &pkg_dir) {
            eprintln!("  error extracting package: {e}");
            return 1;
        }

        // 4. Verify signature
        if !client.verify_signature(&meta, &tarball_bytes) {
            eprintln!("  warning: package signature verification failed");
        }

        // 5. Update galex.toml [dependencies]
        let mut config = GalexConfig::load(project_dir);
        let dep_key = meta.name.replace('/', "-");
        config
            .dependencies
            .insert(dep_key, DependencySpec::Version(meta.version.clone()));
        if let Err(e) = config.save(project_dir) {
            eprintln!("  warning: failed to update galex.toml: {e}");
        }

        // 6. Update gale.lock
        let lock_path = project_dir.join("gale.lock");
        let mut lockfile = Lockfile::load(&lock_path);
        lockfile.add(LockedPackage {
            name: meta.name.replace('/', "-"),
            version: meta.version.clone(),
            checksum: meta.checksum.clone(),
        });
        if let Err(e) = lockfile.save(&lock_path) {
            eprintln!("  warning: failed to update gale.lock: {e}");
        }

        eprintln!("  Added {} v{}", meta.name, meta.version);
        0
    })
}
