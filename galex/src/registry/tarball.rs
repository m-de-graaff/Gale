//! Tarball packing, extraction, and checksum computation.

use std::io::{Read, Write};
use std::path::Path;

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use sha2::{Digest, Sha256};

/// Compute SHA-256 hex digest of data.
pub fn sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// Pack a directory into a `.tar.gz` tarball.
///
/// Includes all files in `dir` with paths relative to `dir`.
pub fn pack(dir: &Path) -> Result<Vec<u8>, std::io::Error> {
    let buf = Vec::new();
    let encoder = GzEncoder::new(buf, Compression::default());
    let mut archive = tar::Builder::new(encoder);

    // Add all files in the directory
    archive.append_dir_all(".", dir)?;
    archive.finish()?;

    let encoder = archive.into_inner()?;
    encoder.finish()
}

/// Extract a `.tar.gz` tarball into a destination directory.
///
/// Creates `dest` if it doesn't exist. Files are extracted with
/// paths relative to `dest`.
pub fn extract(data: &[u8], dest: &Path) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(dest)?;
    let decoder = GzDecoder::new(data);
    let mut archive = tar::Archive::new(decoder);
    archive.unpack(dest)?;
    Ok(())
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_deterministic() {
        let hash1 = sha256(b"hello world");
        let hash2 = sha256(b"hello world");
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // 256 bits = 64 hex chars
    }

    #[test]
    fn sha256_different_inputs() {
        let hash1 = sha256(b"hello");
        let hash2 = sha256(b"world");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn pack_and_extract_roundtrip() {
        let src_dir = std::env::temp_dir().join("gale_tarball_src");
        let dst_dir = std::env::temp_dir().join("gale_tarball_dst");
        let _ = std::fs::remove_dir_all(&src_dir);
        let _ = std::fs::remove_dir_all(&dst_dir);

        // Create source files
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(src_dir.join("hello.txt"), "hello world").unwrap();
        std::fs::create_dir_all(src_dir.join("sub")).unwrap();
        std::fs::write(src_dir.join("sub").join("nested.txt"), "nested content").unwrap();

        // Pack
        let tarball = pack(&src_dir).unwrap();
        assert!(!tarball.is_empty());

        // Extract
        extract(&tarball, &dst_dir).unwrap();

        // Verify
        assert_eq!(
            std::fs::read_to_string(dst_dir.join("hello.txt")).unwrap(),
            "hello world"
        );
        assert_eq!(
            std::fs::read_to_string(dst_dir.join("sub").join("nested.txt")).unwrap(),
            "nested content"
        );

        // Cleanup
        let _ = std::fs::remove_dir_all(&src_dir);
        let _ = std::fs::remove_dir_all(&dst_dir);
    }
}
