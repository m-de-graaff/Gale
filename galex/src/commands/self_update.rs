//! `gale self-update` — download and install the latest gale release.
//!
//! Queries the GitHub releases API, compares the latest tag against the
//! running binary's version, and replaces the binary in-place if a newer
//! release is available.

use std::path::Path;

const GITHUB_REPO: &str = "m-de-graaff/Gale";
const GITHUB_API: &str = "https://api.github.com";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

// ── GitHub API types ───────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(serde::Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

// ── Public entry point ─────────────────────────────────────────────────

/// Run the `gale self-update` command.
pub fn run() -> i32 {
    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    rt.block_on(run_async())
}

// ── Platform helpers ───────────────────────────────────────────────────

/// Return the release asset suffix for the current platform, or `None` if
/// the platform is not a published target.
fn platform_suffix() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Some("linux-x86_64"),
        ("macos", "aarch64") => Some("macos-aarch64"),
        ("macos", "x86_64") => Some("macos-x86_64"),
        ("windows", "x86_64") => Some("windows-x86_64"),
        _ => None,
    }
}

/// Extension of the SDK release archive for this platform.
fn archive_ext() -> &'static str {
    if cfg!(windows) {
        "zip"
    } else {
        "tar.gz"
    }
}

/// Name of the gale CLI binary inside the archive.
fn binary_name() -> &'static str {
    if cfg!(windows) {
        "gale.exe"
    } else {
        "gale"
    }
}

// ── Core async logic ───────────────────────────────────────────────────

async fn run_async() -> i32 {
    eprintln!("  Checking for updates...");

    // ── Detect platform ────────────────────────────────────────
    let suffix = match platform_suffix() {
        Some(s) => s,
        None => {
            eprintln!(
                "  error: unsupported platform ({} {})",
                std::env::consts::OS,
                std::env::consts::ARCH,
            );
            return 1;
        }
    };

    let ext = archive_ext();
    let asset_name = format!("gale-sdk-{suffix}.{ext}");

    // ── Query GitHub releases API ──────────────────────────────
    let client = match reqwest::Client::builder()
        .user_agent(format!("gale/{CURRENT_VERSION}"))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("  error: failed to build HTTP client: {e}");
            return 1;
        }
    };

    let url = format!("{GITHUB_API}/repos/{GITHUB_REPO}/releases/latest");
    let release: Release = match client.get(&url).send().await {
        Ok(r) if r.status().is_success() => match r.json().await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("  error: failed to parse release info: {e}");
                return 1;
            }
        },
        Ok(r) => {
            eprintln!("  error: GitHub API returned {}", r.status());
            return 1;
        }
        Err(e) => {
            eprintln!("  error: {e}");
            return 1;
        }
    };

    // ── Compare versions ───────────────────────────────────────
    let latest_tag = release.tag_name.trim_start_matches('v');

    let up_to_date = match (
        semver::Version::parse(latest_tag),
        semver::Version::parse(CURRENT_VERSION),
    ) {
        (Ok(latest), Ok(current)) => latest <= current,
        _ => false,
    };

    if up_to_date {
        eprintln!("  Already up to date (v{CURRENT_VERSION})");
        return 0;
    }

    // ── Prompt for confirmation ────────────────────────────────
    eprintln!("  New version available: v{latest_tag}  (current: v{CURRENT_VERSION})");
    let confirmed = dialoguer::Confirm::new()
        .with_prompt("  Install update?")
        .default(true)
        .interact()
        .unwrap_or(false);

    if !confirmed {
        eprintln!("  Update cancelled.");
        return 0;
    }

    // ── Find asset download URL ────────────────────────────────
    let asset = match release.assets.iter().find(|a| a.name == asset_name) {
        Some(a) => a,
        None => {
            eprintln!(
                "  error: release v{latest_tag} does not contain an asset named '{asset_name}'"
            );
            return 1;
        }
    };

    eprintln!("  Downloading {asset_name}...");

    // ── Download archive ───────────────────────────────────────
    let bytes = match client.get(&asset.browser_download_url).send().await {
        Ok(r) => match r.bytes().await {
            Ok(b) => b.to_vec(),
            Err(e) => {
                eprintln!("  error: failed to read download: {e}");
                return 1;
            }
        },
        Err(e) => {
            eprintln!("  error: download failed: {e}");
            return 1;
        }
    };

    // ── Extract archive to temp dir ────────────────────────────
    let tmp_dir = std::env::temp_dir().join("gale-self-update");
    let _ = std::fs::remove_dir_all(&tmp_dir);

    if let Err(e) = extract_archive(&bytes, &tmp_dir, ext) {
        eprintln!("  error: failed to extract archive: {e}");
        return 1;
    }

    // ── Locate new binary ──────────────────────────────────────
    let bin = binary_name();
    let new_binary = tmp_dir.join(bin);
    if !new_binary.exists() {
        eprintln!("  error: '{bin}' not found in archive");
        let _ = std::fs::remove_dir_all(&tmp_dir);
        return 1;
    }

    // ── Replace current executable ─────────────────────────────
    let current_exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("  error: cannot determine current executable path: {e}");
            return 1;
        }
    };

    if let Err(e) = replace_binary(&new_binary, &current_exe) {
        eprintln!("  error: failed to install update: {e}");
        eprintln!("  (you may need to re-run with elevated permissions)");
        let _ = std::fs::remove_dir_all(&tmp_dir);
        return 1;
    }

    // ── Also replace gale-lsp if present in the archive ────────
    let lsp_name = if cfg!(windows) {
        "gale-lsp.exe"
    } else {
        "gale-lsp"
    };
    let new_lsp = tmp_dir.join(lsp_name);
    if new_lsp.exists() {
        if let Some(bin_dir) = current_exe.parent() {
            let current_lsp = bin_dir.join(lsp_name);
            if current_lsp.exists() {
                match replace_binary(&new_lsp, &current_lsp) {
                    Ok(()) => eprintln!("  Updated gale-lsp"),
                    Err(e) => eprintln!("  warning: failed to update gale-lsp: {e}"),
                }
            } else {
                // gale-lsp not installed yet — copy it in
                if let Err(e) = std::fs::copy(&new_lsp, &current_lsp) {
                    eprintln!("  warning: failed to install gale-lsp: {e}");
                } else {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let _ = std::fs::set_permissions(
                            &current_lsp,
                            std::fs::Permissions::from_mode(0o755),
                        );
                    }
                    eprintln!("  Installed gale-lsp");
                }
            }
        }
    }

    let _ = std::fs::remove_dir_all(&tmp_dir);
    eprintln!("  Updated to v{latest_tag}  —  run `gale --version` to confirm");
    0
}

// ── Archive extraction ─────────────────────────────────────────────────

fn extract_archive(bytes: &[u8], dest: &Path, ext: &str) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(dest)?;

    if ext == "tar.gz" {
        let decoder = flate2::read::GzDecoder::new(bytes);
        let mut archive = tar::Archive::new(decoder);
        archive.unpack(dest)?;
    } else {
        // .zip  (Windows SDK archive)
        let cursor = std::io::Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(cursor)?;
        archive.extract(dest)?;
    }

    Ok(())
}

// ── Binary replacement ─────────────────────────────────────────────────

/// Replace `current_exe` with `new_binary`.
///
/// - **Unix:** `chmod 0o755` the new file then atomically rename it over the
///   current executable (both paths must be on the same filesystem, which is
///   always the case when `new_binary` lives next to the exe).
/// - **Windows:** Cannot rename over a running `.exe`, so the current binary
///   is first moved to `gale.old.exe` and the new one takes its place.
///   `gale.old.exe` is cleaned up on the next successful self-update.
fn replace_binary(new_binary: &Path, current_exe: &Path) -> std::io::Result<()> {
    // Copy new binary alongside current exe so rename is within one filesystem.
    let staging = current_exe.with_extension("new.exe");
    std::fs::copy(new_binary, &staging)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&staging, std::fs::Permissions::from_mode(0o755))?;
        std::fs::rename(&staging, current_exe)?;
    }

    #[cfg(windows)]
    {
        let old = current_exe.with_extension("old.exe");
        let _ = std::fs::remove_file(&old); // clean up previous stale backup
        std::fs::rename(current_exe, &old)?;
        std::fs::rename(&staging, current_exe)?;
    }

    // Fallback for non-unix non-windows (unreachable in practice given our
    // release targets, but keeps the function total).
    #[cfg(not(any(unix, windows)))]
    {
        std::fs::rename(&staging, current_exe)?;
    }

    Ok(())
}
