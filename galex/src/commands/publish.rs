//! `gale publish` — publish a package to the registry.

use std::path::Path;

use crate::registry::client::RegistryClient;
use crate::registry::package::PackageManifest;
use crate::registry::tarball;

/// Run the `gale publish` command.
pub fn run() -> i32 {
    let project_dir = Path::new(".");
    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");

    // 1. Read gale-package.toml
    let manifest = match PackageManifest::load(project_dir) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("  error: {e}");
            eprintln!("  (is there a gale-package.toml in the current directory?)");
            return 1;
        }
    };
    eprintln!(
        "  Publishing {} v{}...",
        manifest.package.name, manifest.package.version
    );

    // 2. Validate
    if manifest.package.name.is_empty() {
        eprintln!("  error: package name is required");
        return 1;
    }
    if manifest.package.version.is_empty() {
        eprintln!("  error: package version is required");
        return 1;
    }
    if semver::Version::parse(&manifest.package.version).is_err() {
        eprintln!(
            "  error: invalid semver version: {}",
            manifest.package.version
        );
        return 1;
    }

    // 3. Pack into tarball
    let tarball_bytes = match tarball::pack(project_dir) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("  error packing: {e}");
            return 1;
        }
    };
    let checksum = tarball::sha256(&tarball_bytes);
    eprintln!(
        "  Packed {} ({} bytes, checksum: {}...)",
        manifest.package.name,
        tarball_bytes.len(),
        &checksum[..12]
    );

    // 4. Load auth token
    let token = match load_token() {
        Some(t) => t,
        None => {
            eprintln!("  error: not logged in. Run `gale login` first.");
            return 1;
        }
    };

    // 5. Upload to registry
    rt.block_on(async {
        let client = RegistryClient::new();
        match client
            .publish(&manifest, &tarball_bytes, &checksum, &token)
            .await
        {
            Ok(()) => {
                eprintln!(
                    "  Published {} v{}",
                    manifest.package.name, manifest.package.version
                );
                0
            }
            Err(e) => {
                eprintln!("  error: {e}");
                1
            }
        }
    })
}

/// Load the auth token from `~/.gale/credentials`.
fn load_token() -> Option<String> {
    let home = dirs_next::home_dir()?;
    let creds = home.join(".gale").join("credentials");
    std::fs::read_to_string(creds)
        .ok()
        .map(|s| s.trim().to_string())
}
