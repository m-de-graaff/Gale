//! Tailwind CLI integration — runs `npx tailwindcss` to generate CSS.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::config::{self, TailwindConfig};

/// Errors that can occur during Tailwind CSS generation.
#[derive(Debug)]
pub enum TailwindError {
    /// Node.js / npx not found on PATH.
    NodeNotFound,
    /// Tailwind CSS CLI returned a non-zero exit code.
    BuildFailed(String),
    /// I/O error (file read/write).
    IoError(std::io::Error),
}

impl std::fmt::Display for TailwindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TailwindError::NodeNotFound => write!(
                f,
                "Node.js not found. Tailwind CSS requires Node.js.\n\
                 Install from https://nodejs.org, then run: npm install -D tailwindcss"
            ),
            TailwindError::BuildFailed(msg) => write!(f, "Tailwind CSS build failed: {msg}"),
            TailwindError::IoError(e) => write!(f, "I/O error during CSS generation: {e}"),
        }
    }
}

impl From<std::io::Error> for TailwindError {
    fn from(e: std::io::Error) -> Self {
        TailwindError::IoError(e)
    }
}

/// Run the Tailwind CSS CLI to generate an optimized CSS file.
///
/// # Arguments
///
/// * `tw_config` — Parsed Tailwind configuration from `galex.toml`
/// * `app_dir` — Path to the `app/` source directory
/// * `safelist` — Extra class names to always include (from GaleX extraction)
/// * `output_css` — Where to write the generated CSS file
/// * `minify` — Whether to minify the output (production builds)
///
/// # Steps
///
/// 1. Generate `tailwind.config.js` in a temp working directory
/// 2. Generate (or use custom) input CSS with `@tailwind` directives
/// 3. Run `npx tailwindcss --input ... --output ... --config ...`
/// 4. Check exit status and capture stderr for error reporting
pub fn run_tailwind_cli(
    tw_config: &TailwindConfig,
    app_dir: &Path,
    safelist: &[String],
    output_css: &Path,
    minify: bool,
) -> Result<(), TailwindError> {
    // Create a working directory for Tailwind config
    let work_dir = output_css
        .parent()
        .unwrap_or(Path::new("."))
        .join("_gale_tw_work");
    std::fs::create_dir_all(&work_dir)?;

    // Generate tailwind.config.js
    let config_path = config::generate_tailwind_config(tw_config, app_dir, safelist, &work_dir);

    // Generate or use custom input CSS
    let input_css_path = work_dir.join("input.css");
    if let Some(ref custom_css) = tw_config.input_css {
        // Copy user's custom CSS as the input (it should contain @tailwind directives)
        if custom_css.is_file() {
            std::fs::copy(custom_css, &input_css_path)?;
        } else {
            // Custom CSS not found — use default
            std::fs::write(&input_css_path, config::default_input_css())?;
        }
    } else {
        std::fs::write(&input_css_path, config::default_input_css())?;
    }

    // Ensure output directory exists
    if let Some(parent) = output_css.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Build the npx command
    let npx = find_npx()?;
    let mut cmd = Command::new(&npx);
    cmd.arg("tailwindcss");
    cmd.arg("--input").arg(&input_css_path);
    cmd.arg("--output").arg(output_css);
    cmd.arg("--config").arg(&config_path);
    if minify {
        cmd.arg("--minify");
    }

    // Run the command
    let output = cmd.output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            TailwindError::NodeNotFound
        } else {
            TailwindError::IoError(e)
        }
    })?;

    // Clean up working directory
    std::fs::remove_dir_all(&work_dir).ok();

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        Err(TailwindError::BuildFailed(format!(
            "exit code: {}\nstderr: {stderr}\nstdout: {stdout}",
            output.status
        )))
    }
}

/// Find the `npx` executable on PATH.
///
/// On Windows, tries `npx.cmd` first (npm installs .cmd wrappers).
fn find_npx() -> Result<PathBuf, TailwindError> {
    #[cfg(windows)]
    {
        // Try npx.cmd first (standard npm installation on Windows)
        if which_exists("npx.cmd") {
            return Ok(PathBuf::from("npx.cmd"));
        }
    }
    if which_exists("npx") {
        return Ok(PathBuf::from("npx"));
    }
    Err(TailwindError::NodeNotFound)
}

/// Check if a command exists on PATH.
fn which_exists(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tailwind_error_display() {
        let err = TailwindError::NodeNotFound;
        let msg = format!("{err}");
        assert!(msg.contains("Node.js not found"));

        let err = TailwindError::BuildFailed("exit 1".into());
        let msg = format!("{err}");
        assert!(msg.contains("build failed"));
    }

    #[test]
    fn find_npx_does_not_panic() {
        // This just tests that the function doesn't panic.
        // It may return Ok or Err depending on whether Node is installed.
        let _ = find_npx();
    }
}
