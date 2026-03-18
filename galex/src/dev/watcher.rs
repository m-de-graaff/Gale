//! File system watcher with debouncing and change classification.
//!
//! Watches `app/`, `styles/`, `public/`, and `galex.toml` for changes.
//! Classifies each change by type so the rebuild manager can take the
//! minimal action (CSS-only reload, incremental recompile, or full restart).

use std::path::{Path, PathBuf};
use std::sync::mpsc as std_mpsc;
use std::time::Duration;

use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
use tokio::sync::mpsc;

/// Classification of a file system change event.
#[derive(Debug, Clone)]
pub enum ChangeKind {
    /// A `.gx` source file was modified (content change only).
    GxModified(PathBuf),
    /// A `.gx` source file was created or deleted (route structure may have changed).
    GxStructural(PathBuf),
    /// A CSS/style file changed.
    CssChanged(PathBuf),
    /// A static asset in `public/` changed.
    AssetChanged(PathBuf),
    /// `galex.toml` config changed — requires full restart.
    ConfigChanged,
}

impl ChangeKind {
    /// Get the file path associated with this change, if any.
    pub fn path(&self) -> Option<&Path> {
        match self {
            ChangeKind::GxModified(p)
            | ChangeKind::GxStructural(p)
            | ChangeKind::CssChanged(p)
            | ChangeKind::AssetChanged(p) => Some(p),
            ChangeKind::ConfigChanged => None,
        }
    }
}

/// File watcher that produces batched, classified change events.
pub struct FileWatcher {
    rx: mpsc::Receiver<Vec<ChangeKind>>,
    // Keep the debouncer alive — dropping it stops watching.
    _debouncer: notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>,
}

/// Special file names that affect route structure when created/deleted.
const STRUCTURAL_FILES: &[&str] = &[
    "page.gx",
    "layout.gx",
    "guard.gx",
    "middleware.gx",
    "error.gx",
    "loading.gx",
];

impl FileWatcher {
    /// Start watching the given directories for changes.
    ///
    /// - `app_dir` — the `app/` source directory (recursive)
    /// - `project_dir` — the project root (for `galex.toml`, `public/`, `styles/`)
    pub fn start(app_dir: &Path, project_dir: &Path) -> Result<Self, notify::Error> {
        let (std_tx, std_rx) = std_mpsc::channel();
        let mut debouncer = new_debouncer(Duration::from_millis(50), std_tx)?;

        let watcher = debouncer.watcher();

        // Watch app/ recursively
        if app_dir.is_dir() {
            watcher.watch(app_dir, RecursiveMode::Recursive)?;
        }

        // Watch public/ if it exists
        let public_dir = project_dir.join("public");
        if public_dir.is_dir() {
            watcher.watch(&public_dir, RecursiveMode::Recursive)?;
        }

        // Watch styles/ if it exists
        let styles_dir = project_dir.join("styles");
        if styles_dir.is_dir() {
            watcher.watch(&styles_dir, RecursiveMode::Recursive)?;
        }

        // Watch galex.toml
        let config_file = project_dir.join("galex.toml");
        if config_file.is_file() {
            watcher.watch(&config_file, RecursiveMode::NonRecursive)?;
        }

        // Bridge from std::sync::mpsc to tokio::sync::mpsc
        let app_dir_owned = app_dir.to_path_buf();
        let project_dir_owned = project_dir.to_path_buf();
        let (tx, rx) = mpsc::channel(32);

        std::thread::spawn(move || {
            while let Ok(result) = std_rx.recv() {
                match result {
                    Ok(events) => {
                        let changes: Vec<ChangeKind> = events
                            .into_iter()
                            .filter_map(|ev| {
                                classify_change(
                                    &ev.path,
                                    &ev.kind,
                                    &app_dir_owned,
                                    &project_dir_owned,
                                )
                            })
                            .collect();
                        if !changes.is_empty() {
                            if tx.blocking_send(changes).is_err() {
                                break;
                            }
                        }
                    }
                    Err(_) => {} // Watch error — ignore
                }
            }
        });

        Ok(Self {
            rx,
            _debouncer: debouncer,
        })
    }

    /// Wait for the next batch of classified changes.
    ///
    /// Returns `None` if the watcher was dropped.
    pub async fn next_changes(&mut self) -> Option<Vec<ChangeKind>> {
        self.rx.recv().await
    }
}

/// Classify a single file change event.
fn classify_change(
    path: &Path,
    kind: &DebouncedEventKind,
    app_dir: &Path,
    project_dir: &Path,
) -> Option<ChangeKind> {
    let _ = kind; // We use path-based classification, not event kind

    // galex.toml config
    if path.ends_with("galex.toml") {
        return Some(ChangeKind::ConfigChanged);
    }

    // Check if under app/ directory
    if path.starts_with(app_dir) {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext == "gx" {
            // Check if a structural file was created or deleted
            // (We detect this by checking the event kind — Any covers both
            // modify and create, so we rely on file existence for deletes)
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if STRUCTURAL_FILES.contains(&file_name) {
                if matches!(kind, DebouncedEventKind::AnyContinuous) || !path.exists() {
                    // File was deleted or rename detected — structural change
                    return Some(ChangeKind::GxStructural(path.to_path_buf()));
                }
            }
            // Content modification (or structural file still exists = just modified)
            return Some(ChangeKind::GxModified(path.to_path_buf()));
        }
        if ext == "css" {
            return Some(ChangeKind::CssChanged(path.to_path_buf()));
        }
    }

    // Check if under styles/ directory
    let styles_dir = project_dir.join("styles");
    if path.starts_with(&styles_dir) {
        return Some(ChangeKind::CssChanged(path.to_path_buf()));
    }

    // Check if under public/ directory
    let public_dir = project_dir.join("public");
    if path.starts_with(&public_dir) {
        return Some(ChangeKind::AssetChanged(path.to_path_buf()));
    }

    None // Unknown file — ignore
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> PathBuf {
        std::env::temp_dir().join("gale_test_project").join("app")
    }
    fn proj() -> PathBuf {
        std::env::temp_dir().join("gale_test_project")
    }

    #[test]
    fn classify_gx_modified() {
        // Use a non-structural .gx file (not page.gx/layout.gx etc.)
        let change = classify_change(
            &app().join("components.gx"),
            &DebouncedEventKind::Any,
            &app(),
            &proj(),
        );
        assert!(
            matches!(change, Some(ChangeKind::GxModified(_))),
            "expected GxModified, got {change:?}"
        );
    }

    #[test]
    fn classify_structural_gx_deleted() {
        // A structural file (page.gx) that doesn't exist on disk → GxStructural
        let change = classify_change(
            &app().join("nonexistent_dir").join("page.gx"),
            &DebouncedEventKind::Any,
            &app(),
            &proj(),
        );
        assert!(
            matches!(change, Some(ChangeKind::GxStructural(_))),
            "expected GxStructural for deleted page.gx, got {change:?}"
        );
    }

    #[test]
    fn classify_css_in_styles() {
        let change = classify_change(
            &proj().join("styles").join("global.css"),
            &DebouncedEventKind::Any,
            &app(),
            &proj(),
        );
        assert!(matches!(change, Some(ChangeKind::CssChanged(_))));
    }

    #[test]
    fn classify_css_in_app() {
        let change = classify_change(
            &app().join("components.css"),
            &DebouncedEventKind::Any,
            &app(),
            &proj(),
        );
        assert!(matches!(change, Some(ChangeKind::CssChanged(_))));
    }

    #[test]
    fn classify_asset() {
        let change = classify_change(
            &proj().join("public").join("logo.png"),
            &DebouncedEventKind::Any,
            &app(),
            &proj(),
        );
        assert!(matches!(change, Some(ChangeKind::AssetChanged(_))));
    }

    #[test]
    fn classify_config() {
        let change = classify_change(
            &proj().join("galex.toml"),
            &DebouncedEventKind::Any,
            &app(),
            &proj(),
        );
        assert!(matches!(change, Some(ChangeKind::ConfigChanged)));
    }

    #[test]
    fn classify_unknown_ignored() {
        let change = classify_change(
            Path::new("C:\\other\\random.txt"),
            &DebouncedEventKind::Any,
            &app(),
            &proj(),
        );
        assert!(change.is_none());
    }
}
