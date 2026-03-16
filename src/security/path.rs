use std::path::PathBuf;

use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use percent_encoding::percent_decode_str;

use crate::error::ErrorResponse;

#[derive(Clone)]
pub struct PathSecurityState {
    pub canonical_root: PathBuf,
    pub block_dotfiles: bool,
}

pub async fn path_security_middleware(
    State(state): State<PathSecurityState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, ErrorResponse> {
    let raw_path = request.uri().path();

    validate_path(raw_path, &state).map_err(|status| {
        tracing::warn!(status = status.as_u16(), path = raw_path, "request rejected by path security");
        ErrorResponse::new(status)
    })?;

    Ok(next.run(request).await)
}

fn validate_path(raw_path: &str, state: &PathSecurityState) -> Result<(), StatusCode> {
    // Step 1: Reject null bytes (literal or encoded)
    if raw_path.contains('\0') || raw_path.contains("%00") || raw_path.contains("%2500") {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Step 2: Percent-decode the path once
    let decoded = percent_decode_str(raw_path)
        .decode_utf8()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // Step 3: Reject double-encoding — after one decode, no encoded traversal chars should remain
    let lower = decoded.to_lowercase();
    if lower.contains("%2e") || lower.contains("%2f") || lower.contains("%5c") {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Normalize backslashes to forward slashes for uniform handling
    let normalized = decoded.replace('\\', "/");

    // Step 4: Reject traversal patterns — ".." as a path segment
    for segment in normalized.split('/') {
        if segment == ".." {
            return Err(StatusCode::FORBIDDEN);
        }
    }

    // Step 5: Windows-specific attack vectors
    #[cfg(windows)]
    validate_windows(&normalized)?;

    // Step 6: Dotfile blocking
    if state.block_dotfiles {
        for segment in normalized.split('/') {
            if !segment.is_empty() && segment.starts_with('.') {
                return Err(StatusCode::FORBIDDEN);
            }
        }
    }

    // Step 7: Symlink jail enforcement
    // Construct the filesystem path and verify it resolves within the root
    let relative = normalized.trim_start_matches('/');
    if !relative.is_empty() {
        let candidate = state.canonical_root.join(relative);
        if let Ok(resolved) = candidate.canonicalize() {
            if !resolved.starts_with(&state.canonical_root) {
                return Err(StatusCode::FORBIDDEN);
            }

            // On Windows with block_dotfiles, also check hidden file attribute on ancestors
            #[cfg(windows)]
            if state.block_dotfiles {
                let mut check = resolved.as_path();
                while check.starts_with(&state.canonical_root) && check != state.canonical_root {
                    if crate::platform::is_hidden(check) {
                        return Err(StatusCode::FORBIDDEN);
                    }
                    match check.parent() {
                        Some(parent) => check = parent,
                        None => break,
                    }
                }
            }
        }
        // If canonicalize fails, file doesn't exist — let ServeDir handle 404
    }

    Ok(())
}

#[cfg(windows)]
fn validate_windows(normalized: &str) -> Result<(), StatusCode> {
    // Reject UNC paths
    if normalized.starts_with("//") {
        return Err(StatusCode::FORBIDDEN);
    }

    for segment in normalized.split('/') {
        if segment.is_empty() {
            continue;
        }

        // Reject Alternate Data Streams (colon in segment)
        if segment.contains(':') {
            return Err(StatusCode::FORBIDDEN);
        }

        // Reject reserved device names (CON, PRN, NUL, AUX, COM1-9, LPT1-9)
        // These are reserved with or without extensions (e.g., CON.txt)
        let stem = segment.split('.').next().unwrap_or(segment);
        let upper = stem.to_uppercase();
        if matches!(
            upper.as_str(),
            "CON"
                | "PRN"
                | "NUL"
                | "AUX"
                | "COM1"
                | "COM2"
                | "COM3"
                | "COM4"
                | "COM5"
                | "COM6"
                | "COM7"
                | "COM8"
                | "COM9"
                | "LPT1"
                | "LPT2"
                | "LPT3"
                | "LPT4"
                | "LPT5"
                | "LPT6"
                | "LPT7"
                | "LPT8"
                | "LPT9"
        ) {
            return Err(StatusCode::FORBIDDEN);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_with_dotfiles(block: bool) -> PathSecurityState {
        // Use a canonical path that exists on the system
        let root = std::env::current_dir().expect("failed to get cwd");
        PathSecurityState {
            canonical_root: root,
            block_dotfiles: block,
        }
    }

    fn state() -> PathSecurityState {
        state_with_dotfiles(true)
    }

    // --- Traversal ---

    #[test]
    fn rejects_dot_dot_slash() {
        assert_eq!(validate_path("/../etc/passwd", &state()), Err(StatusCode::FORBIDDEN));
    }

    #[test]
    fn rejects_encoded_traversal() {
        assert_eq!(validate_path("/%2e%2e/etc/passwd", &state()), Err(StatusCode::FORBIDDEN));
    }

    #[test]
    fn rejects_double_encoded_traversal() {
        // %252e decodes to %2e on first pass, which we reject as double-encoding
        assert_eq!(
            validate_path("/%252e%252e/Cargo.toml", &state()),
            Err(StatusCode::BAD_REQUEST)
        );
    }

    #[test]
    fn rejects_backslash_traversal() {
        assert_eq!(validate_path("/..\\etc\\passwd", &state()), Err(StatusCode::FORBIDDEN));
    }

    #[test]
    fn rejects_encoded_backslash_traversal() {
        assert_eq!(
            validate_path("/%2e%2e%5cetc%5cpasswd", &state()),
            Err(StatusCode::FORBIDDEN)
        );
    }

    // --- Null bytes ---

    #[test]
    fn rejects_null_byte_encoded() {
        assert_eq!(validate_path("/file%00.txt", &state()), Err(StatusCode::BAD_REQUEST));
    }

    #[test]
    fn rejects_null_byte_literal() {
        assert_eq!(validate_path("/file\0.txt", &state()), Err(StatusCode::BAD_REQUEST));
    }

    // --- Dotfiles ---

    #[test]
    fn rejects_dotfile_root() {
        assert_eq!(validate_path("/.env", &state()), Err(StatusCode::FORBIDDEN));
    }

    #[test]
    fn rejects_dotfile_nested() {
        assert_eq!(
            validate_path("/assets/.git/config", &state()),
            Err(StatusCode::FORBIDDEN)
        );
    }

    #[test]
    fn allows_dotfile_when_disabled() {
        let s = state_with_dotfiles(false);
        assert!(validate_path("/.env", &s).is_ok());
    }

    // --- Windows-specific ---

    #[cfg(windows)]
    #[test]
    fn rejects_ads() {
        assert_eq!(
            validate_path("/file.txt:hidden", &state()),
            Err(StatusCode::FORBIDDEN)
        );
    }

    #[cfg(windows)]
    #[test]
    fn rejects_reserved_device_con() {
        assert_eq!(validate_path("/CON", &state()), Err(StatusCode::FORBIDDEN));
    }

    #[cfg(windows)]
    #[test]
    fn rejects_reserved_device_con_with_ext() {
        assert_eq!(validate_path("/CON.txt", &state()), Err(StatusCode::FORBIDDEN));
    }

    #[cfg(windows)]
    #[test]
    fn rejects_reserved_device_nul() {
        assert_eq!(validate_path("/NUL", &state()), Err(StatusCode::FORBIDDEN));
    }

    #[cfg(windows)]
    #[test]
    fn rejects_unc_path() {
        assert_eq!(
            validate_path("//server/share", &state()),
            Err(StatusCode::FORBIDDEN)
        );
    }

    // --- Valid paths ---

    #[test]
    fn allows_normal_path() {
        assert!(validate_path("/index.html", &state()).is_ok());
    }

    #[test]
    fn allows_nested_path() {
        assert!(validate_path("/assets/css/style.css", &state()).is_ok());
    }

    #[test]
    fn allows_percent_encoded_space() {
        assert!(validate_path("/hello%20world.html", &state()).is_ok());
    }

    #[test]
    fn allows_root_path() {
        assert!(validate_path("/", &state()).is_ok());
    }
}
