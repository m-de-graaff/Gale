//! HTTP client for the GaleX package registry.

use std::path::Path;

use super::{PackageMeta, RegistryError};

const DEFAULT_REGISTRY: &str = "https://registry.get-gale.vercel.app";

/// HTTP client for the GaleX package registry.
pub struct RegistryClient {
    http: reqwest::Client,
    registry_url: String,
}

impl RegistryClient {
    /// Create a new registry client.
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
            registry_url: std::env::var("GALE_REGISTRY")
                .unwrap_or_else(|_| DEFAULT_REGISTRY.to_string()),
        }
    }

    /// Fetch package metadata from the registry.
    pub async fn fetch_meta(&self, name: &str) -> Result<PackageMeta, RegistryError> {
        let url = format!("{}/api/packages/{}", self.registry_url, name);
        let response = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| RegistryError::Network(e.to_string()))?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(RegistryError::NotFound(name.to_string()));
        }

        if !response.status().is_success() {
            return Err(RegistryError::Network(format!(
                "registry returned status {}",
                response.status()
            )));
        }

        response
            .json::<PackageMeta>()
            .await
            .map_err(|e| RegistryError::Network(e.to_string()))
    }

    /// Download a package tarball to the given directory.
    pub async fn download(
        &self,
        meta: &PackageMeta,
        dest_dir: &Path,
    ) -> Result<Vec<u8>, RegistryError> {
        let response = self
            .http
            .get(&meta.download_url)
            .send()
            .await
            .map_err(|e| RegistryError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(RegistryError::Network(format!(
                "download failed with status {}",
                response.status()
            )));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| RegistryError::Network(e.to_string()))?;

        // Verify checksum
        let actual_hash = sha256_hex(&bytes);
        if actual_hash != meta.checksum {
            return Err(RegistryError::ChecksumMismatch);
        }

        // Write to dest_dir/{package_name}/
        let pkg_dir = dest_dir.join(meta.name.replace('/', "_"));
        std::fs::create_dir_all(&pkg_dir)?;
        std::fs::write(pkg_dir.join("package.tar.gz"), &bytes)?;

        Ok(bytes.to_vec())
    }

    /// Verify the package signature (if signed).
    pub fn verify_signature(&self, meta: &PackageMeta, _tarball: &[u8]) -> bool {
        // TODO: Ed25519 signature verification
        // For now, accept all packages (signature verification requires
        // a public key registry which is not yet implemented)
        meta.signature.is_none() // Unsigned packages pass, signed ones need verification
    }

    /// Publish a package to the registry.
    pub async fn publish(
        &self,
        manifest: &super::package::PackageManifest,
        tarball: &[u8],
        checksum: &str,
        token: &str,
    ) -> Result<(), RegistryError> {
        let url = format!("{}/api/packages", self.registry_url);
        let form = reqwest::multipart::Form::new()
            .text("name", manifest.package.name.clone())
            .text("version", manifest.package.version.clone())
            .text("description", manifest.package.description.clone())
            .text("checksum", checksum.to_string())
            .part(
                "tarball",
                reqwest::multipart::Part::bytes(tarball.to_vec())
                    .file_name("package.tar.gz")
                    .mime_str("application/gzip")
                    .unwrap(),
            );

        let response = self
            .http
            .post(&url)
            .bearer_auth(token)
            .multipart(form)
            .send()
            .await
            .map_err(|e| RegistryError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(RegistryError::Network(format!("publish failed: {body}")));
        }

        Ok(())
    }

    /// Search for packages in the registry.
    pub async fn search(&self, query: &str) -> Result<Vec<PackageMeta>, RegistryError> {
        let url = format!(
            "{}/api/search?q={}",
            self.registry_url,
            urlencoding::encode(query)
        );
        let response = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| RegistryError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(RegistryError::Network(format!(
                "search failed: {}",
                response.status()
            )));
        }

        response
            .json::<Vec<PackageMeta>>()
            .await
            .map_err(|e| RegistryError::Network(e.to_string()))
    }
}

impl Default for RegistryClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute SHA-256 hex digest using the real sha2 crate.
fn sha256_hex(data: &[u8]) -> String {
    super::tarball::sha256(data)
}
