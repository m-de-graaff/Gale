//! `gale remove` — remove a package.

use std::path::Path;

use crate::config::GalexConfig;
use crate::registry::lockfile::Lockfile;

/// Run the `gale remove` command.
pub fn run(package: &str) -> i32 {
    let project_dir = Path::new(".");
    let dep_key = package.replace('/', "-");

    // 1. Remove from galex.toml [dependencies]
    let mut config = GalexConfig::load(project_dir);
    if config.dependencies.remove(&dep_key).is_none() {
        eprintln!("  Package '{package}' is not in dependencies");
        return 1;
    }
    if let Err(e) = config.save(project_dir) {
        eprintln!("  error updating galex.toml: {e}");
        return 1;
    }

    // 2. Remove from gale.lock
    let lock_path = project_dir.join("gale.lock");
    let mut lockfile = Lockfile::load(&lock_path);
    lockfile.packages.retain(|p| p.name != dep_key);
    if let Err(e) = lockfile.save(&lock_path) {
        eprintln!("  warning: failed to update gale.lock: {e}");
    }

    // 3. Remove from gale_modules/
    let pkg_dir = project_dir
        .join("gale_modules")
        .join(package.replace('/', "_"));
    if pkg_dir.is_dir() {
        if let Err(e) = std::fs::remove_dir_all(&pkg_dir) {
            eprintln!("  warning: failed to remove {}: {e}", pkg_dir.display());
        }
    }

    eprintln!("  Removed {package}");
    0
}
