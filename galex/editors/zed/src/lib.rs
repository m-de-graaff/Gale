use std::fs;
use zed_extension_api::{self as zed, LanguageServerId, Result};

struct GaleExtension {
    cached_binary_path: Option<String>,
}

impl GaleExtension {
    /// Returns the platform-specific asset name for GitHub releases.
    fn platform_asset_name() -> std::result::Result<String, String> {
        let (os, arch) = zed::current_platform();
        let os_str = match os {
            zed::Os::Linux => "unknown-linux-musl",
            zed::Os::Mac => "apple-darwin",
            zed::Os::Windows => "pc-windows-msvc",
        };
        let arch_str = match arch {
            zed::Architecture::Aarch64 => "aarch64",
            zed::Architecture::X8664 => "x86_64",
            zed::Architecture::X86 => return Err("32-bit x86 is not supported".into()),
        };
        let ext = match os {
            zed::Os::Windows => ".exe",
            _ => "",
        };
        Ok(format!("gale-lsp-{arch_str}-{os_str}{ext}"))
    }

    /// Returns the path to the language server binary, downloading it if needed.
    fn language_server_binary_path(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<String> {
        // 1. Check if gale-lsp is on PATH
        if let Some(path) = worktree.which("gale-lsp") {
            return Ok(path);
        }

        // 2. Check cached binary from a previous download
        if let Some(path) = &self.cached_binary_path {
            if fs::metadata(path).map_or(false, |m| m.is_file()) {
                return Ok(path.clone());
            }
        }

        // 3. Download from the latest GitHub release
        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        let release = zed::latest_github_release(
            "m-de-graaff/Gale",
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;

        let asset_name = Self::platform_asset_name()?;
        let asset = release
            .assets
            .iter()
            .find(|a| a.name == asset_name)
            .ok_or_else(|| {
                format!(
                    "no release asset found for {asset_name} in release {}",
                    release.version
                )
            })?;

        let version_dir = format!("gale-lsp-{}", release.version);
        let binary_name = if matches!(zed::current_platform().0, zed::Os::Windows) {
            "gale-lsp.exe"
        } else {
            "gale-lsp"
        };
        let binary_path = format!("{version_dir}/{binary_name}");

        if !fs::metadata(&binary_path).map_or(false, |m| m.is_file()) {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            fs::create_dir_all(&version_dir)
                .map_err(|e| format!("failed to create directory {version_dir}: {e}"))?;

            zed::download_file(
                &asset.download_url,
                &binary_path,
                zed::DownloadedFileType::Uncompressed,
            )
            .map_err(|e| format!("failed to download gale-lsp: {e}"))?;

            zed::make_file_executable(&binary_path)
                .map_err(|e| format!("failed to make gale-lsp executable: {e}"))?;
        }

        self.cached_binary_path = Some(binary_path.clone());
        Ok(binary_path)
    }
}

impl zed::Extension for GaleExtension {
    fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let binary_path = self.language_server_binary_path(language_server_id, worktree)?;
        Ok(zed::Command {
            command: binary_path,
            args: vec![],
            env: Default::default(),
        })
    }

    fn language_server_initialization_options(
        &mut self,
        _language_server_id: &LanguageServerId,
        _worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        Ok(Some(zed::serde_json::json!({
            "diagnostics": { "enabled": true },
            "formatting": { "enabled": true },
            "completion": { "snippets": true },
            "semanticTokens": { "enabled": true }
        })))
    }

    fn language_server_workspace_configuration(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        let settings = zed::settings::LspSettings::for_worktree("gale-lsp", worktree)
            .ok()
            .and_then(|s| s.settings);
        Ok(settings)
    }
}

zed::register_extension!(GaleExtension);
