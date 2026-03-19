//! `gale editor install` — download and install editor extensions.
//!
//! Usage:
//!   gale editor install vscode
//!   gale editor install zed

use std::process::Command;

const GITHUB_REPO: &str = "m-de-graaff/Gale";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Run the `gale editor install <editor>` command.
pub fn run_install(editor: &str) -> i32 {
    match editor.to_lowercase().as_str() {
        "vscode" | "code" => install_vscode(),
        "zed" => install_zed(),
        other => {
            eprintln!("  error: unknown editor '{other}'");
            eprintln!("  Supported editors: vscode, zed");
            1
        }
    }
}

// ── VS Code ────────────────────────────────────────────────────────────

fn install_vscode() -> i32 {
    // Check `code` is available before downloading anything.
    if !command_exists("code") {
        eprintln!("  error: 'code' command not found on PATH");
        eprintln!("  Install VS Code and ensure the 'code' CLI is available, then retry.");
        return 1;
    }

    let vsix_name = format!("gale-vscode-{CURRENT_VERSION}.vsix");
    let download_url = format!(
        "https://github.com/{GITHUB_REPO}/releases/download/v{CURRENT_VERSION}/{vsix_name}"
    );

    eprintln!("  Installing Gale VS Code extension v{CURRENT_VERSION}...");
    eprintln!("  Downloading {vsix_name}...");

    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    let bytes = rt.block_on(async {
        let client = reqwest::Client::builder()
            .user_agent(format!("gale/{CURRENT_VERSION}"))
            .build()
            .unwrap();

        client
            .get(&download_url)
            .send()
            .await
            .map_err(|e| e.to_string())
            .and_then(|r| {
                if r.status().is_success() {
                    Ok(r)
                } else {
                    Err(format!("HTTP {}", r.status()))
                }
            })
    });

    let response = match bytes {
        Ok(r) => r,
        Err(e) => {
            eprintln!("  error: download failed: {e}");
            eprintln!("  URL: {download_url}");
            return 1;
        }
    };

    let bytes = rt.block_on(async { response.bytes().await });
    let bytes = match bytes {
        Ok(b) => b,
        Err(e) => {
            eprintln!("  error: failed to read download: {e}");
            return 1;
        }
    };

    // Write vsix to a temp file.
    let tmp_path = std::env::temp_dir().join(&vsix_name);
    if let Err(e) = std::fs::write(&tmp_path, &bytes) {
        eprintln!("  error: failed to write temp file: {e}");
        return 1;
    }

    // Run: code --install-extension <path>
    eprintln!("  Running: code --install-extension {}", tmp_path.display());
    let status = Command::new("code")
        .arg("--install-extension")
        .arg(&tmp_path)
        .arg("--force")
        .status();

    let _ = std::fs::remove_file(&tmp_path);

    match status {
        Ok(s) if s.success() => {
            eprintln!("  Gale VS Code extension installed.");
            0
        }
        Ok(s) => {
            eprintln!("  error: 'code --install-extension' exited with {s}");
            1
        }
        Err(e) => {
            eprintln!("  error: failed to run 'code': {e}");
            1
        }
    }
}

// ── Zed ───────────────────────────────────────────────────────────────

fn install_zed() -> i32 {
    let zip_name = format!("gale-zed-{CURRENT_VERSION}.zip");
    let download_url =
        format!("https://github.com/{GITHUB_REPO}/releases/download/v{CURRENT_VERSION}/{zip_name}");

    eprintln!("  Installing Gale Zed extension v{CURRENT_VERSION}...");
    eprintln!("  Downloading {zip_name}...");

    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    let bytes = rt.block_on(async {
        let client = reqwest::Client::builder()
            .user_agent(format!("gale/{CURRENT_VERSION}"))
            .build()
            .unwrap();
        client
            .get(&download_url)
            .send()
            .await
            .map_err(|e| e.to_string())
            .and_then(|r| {
                if r.status().is_success() {
                    Ok(r)
                } else {
                    Err(format!("HTTP {}", r.status()))
                }
            })
    });

    let response = match bytes {
        Ok(r) => r,
        Err(e) => {
            eprintln!("  error: download failed: {e}");
            eprintln!("  URL: {download_url}");
            return 1;
        }
    };

    let bytes = rt.block_on(async { response.bytes().await });
    let bytes = match bytes {
        Ok(b) => b,
        Err(e) => {
            eprintln!("  error: failed to read download: {e}");
            return 1;
        }
    };

    // Extract zip to temp dir.
    let tmp_dir = std::env::temp_dir().join("gale-zed-install");
    let _ = std::fs::remove_dir_all(&tmp_dir);
    let cursor = std::io::Cursor::new(bytes.as_ref());
    let mut archive = match zip::ZipArchive::new(cursor) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("  error: failed to open zip: {e}");
            return 1;
        }
    };
    if let Err(e) = archive.extract(&tmp_dir) {
        eprintln!("  error: failed to extract zip: {e}");
        return 1;
    }

    // The zip contains a single `gale-zed/` directory. Run the install script.
    let ext_dir = tmp_dir.join("gale-zed");

    #[cfg(windows)]
    let result = run_zed_install_windows(&ext_dir);
    #[cfg(not(windows))]
    let result = run_zed_install_unix(&ext_dir);

    let _ = std::fs::remove_dir_all(&tmp_dir);
    result
}

#[cfg(windows)]
fn run_zed_install_windows(ext_dir: &std::path::Path) -> i32 {
    let script = ext_dir.join("install-zed.ps1");
    if !script.exists() {
        eprintln!("  error: install-zed.ps1 not found in bundle");
        return 1;
    }
    match Command::new("powershell")
        .args(["-ExecutionPolicy", "Bypass", "-File"])
        .arg(&script)
        .current_dir(ext_dir)
        .status()
    {
        Ok(s) if s.success() => {
            eprintln!("  Gale Zed extension installed. Reload Zed to activate.");
            0
        }
        Ok(s) => {
            eprintln!("  error: install script exited with {s}");
            1
        }
        Err(e) => {
            eprintln!("  error: failed to run install script: {e}");
            1
        }
    }
}

#[cfg(not(windows))]
fn run_zed_install_unix(ext_dir: &std::path::Path) -> i32 {
    let script = ext_dir.join("install-zed.sh");
    if !script.exists() {
        eprintln!("  error: install-zed.sh not found in bundle");
        return 1;
    }
    // Ensure script is executable.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755));
    }
    match Command::new("bash")
        .arg(&script)
        .current_dir(ext_dir)
        .status()
    {
        Ok(s) if s.success() => {
            eprintln!("  Gale Zed extension installed. Reload Zed to activate.");
            0
        }
        Ok(s) => {
            eprintln!("  error: install script exited with {s}");
            1
        }
        Err(e) => {
            eprintln!("  error: failed to run install script: {e}");
            1
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────

fn command_exists(cmd: &str) -> bool {
    // On Windows try `where`, on Unix try `which`.
    #[cfg(windows)]
    let result = Command::new("where").arg(cmd).output();
    #[cfg(not(windows))]
    let result = Command::new("which").arg(cmd).output();

    result.map(|o| o.status.success()).unwrap_or(false)
}
