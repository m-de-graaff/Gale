use std::path::Path;

/// Returns `true` if the given path's filename is hidden.
///
/// On all platforms, a file starting with `.` is considered hidden.
/// On Windows, additionally checks the `FILE_ATTRIBUTE_HIDDEN` flag.
#[inline]
pub fn is_hidden(path: &Path) -> bool {
    let name = match path.file_name().and_then(|n| n.to_str()) {
        Some(n) => n,
        None => return false,
    };

    if name.starts_with('.') {
        return true;
    }

    #[cfg(windows)]
    {
        if let Ok(metadata) = std::fs::metadata(path) {
            use std::os::windows::fs::MetadataExt;
            const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
            if metadata.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0 {
                return true;
            }
        }
    }

    false
}

pub async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm =
            signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");
        let mut sigint = signal(SignalKind::interrupt()).expect("failed to install SIGINT handler");
        tokio::select! {
            _ = sigterm.recv() => {
                tracing::info!("received SIGTERM, shutting down gracefully");
            }
            _ = sigint.recv() => {
                tracing::info!("received SIGINT, shutting down gracefully");
            }
        }
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
        tracing::info!("received Ctrl+C, shutting down gracefully");
    }
}
