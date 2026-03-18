//! Package registry client for `gale add`.

pub mod client;
pub mod lockfile;
pub mod package;
pub mod resolve;
pub mod tarball;

/// Package metadata from the registry.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct PackageMeta {
    /// Package name (e.g. "db/postgres").
    pub name: String,
    /// Latest version (e.g. "1.2.0").
    pub version: String,
    /// SHA-256 checksum of the tarball.
    pub checksum: String,
    /// Ed25519 signature (optional).
    pub signature: Option<String>,
    /// Dependencies on other packages.
    pub dependencies: Vec<String>,
    /// Download URL.
    pub download_url: String,
}

/// Error from registry operations.
#[derive(Debug)]
pub enum RegistryError {
    /// Network error.
    Network(String),
    /// Package not found.
    NotFound(String),
    /// Checksum mismatch.
    ChecksumMismatch,
    /// I/O error.
    Io(std::io::Error),
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistryError::Network(msg) => write!(f, "network error: {msg}"),
            RegistryError::NotFound(name) => write!(f, "package not found: {name}"),
            RegistryError::ChecksumMismatch => write!(f, "checksum mismatch — download corrupted"),
            RegistryError::Io(e) => write!(f, "I/O error: {e}"),
        }
    }
}

impl From<std::io::Error> for RegistryError {
    fn from(e: std::io::Error) -> Self {
        RegistryError::Io(e)
    }
}
