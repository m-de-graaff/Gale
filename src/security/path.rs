use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use dashmap::DashMap;
use percent_encoding::percent_decode_str;

use crate::error::ErrorResponse;

#[derive(Clone)]
pub struct PathSecurityState {
    pub canonical_root: PathBuf,
    pub block_dotfiles: bool,
    /// Cache of joined path → canonicalized path.  Eliminates the expensive
    /// `canonicalize()` syscall on repeat requests for the same file.
    pub canonical_cache: Arc<DashMap<PathBuf, PathBuf>>,
}

pub async fn path_security_middleware(
    State(state): State<PathSecurityState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, ErrorResponse> {
    let raw_path = request.uri().path();

    validate_path(raw_path, &state).map_err(|status| {
        // Suppress noisy warnings for well-known browser probes
        // (Chrome DevTools, service worker manifests, etc.)
        if !raw_path.starts_with("/.well-known/") {
            tracing::warn!(
                status = status.as_u16(),
                path = raw_path,
                "request rejected by path security"
            );
        }
        ErrorResponse::new(status)
    })?;

    Ok(next.run(request).await)
}

#[inline]
pub fn validate_path(raw_path: &str, state: &PathSecurityState) -> Result<(), StatusCode> {
    // Step 1: Reject null bytes (literal or encoded)
    if raw_path.contains('\0') || raw_path.contains("%00") || raw_path.contains("%2500") {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Step 2: Percent-decode the path once
    let decoded = percent_decode_str(raw_path)
        .decode_utf8()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // Step 3: Reject double-encoding — byte-level check without allocating.
    // Previous: decoded.to_lowercase() allocated a new String on every request.
    if has_encoded_traversal(&decoded) {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Normalize backslashes — Cow avoids allocation when no backslashes present
    // (the common case on Linux/macOS).
    let normalized = if decoded.contains('\\') {
        std::borrow::Cow::Owned(decoded.replace('\\', "/"))
    } else {
        std::borrow::Cow::Borrowed(decoded.as_ref())
    };

    // Step 4+6: Single pass — check traversal AND dotfiles in one scan
    for segment in normalized.split('/') {
        if segment == ".." {
            return Err(StatusCode::FORBIDDEN);
        }
        if state.block_dotfiles && !segment.is_empty() && segment.starts_with('.') {
            return Err(StatusCode::FORBIDDEN);
        }
    }

    // Step 5: Windows-specific attack vectors
    #[cfg(windows)]
    validate_windows(&normalized)?;

    // Step 7: Symlink jail enforcement
    // Construct the filesystem path and verify it resolves within the root.
    // Uses a DashMap cache to avoid the expensive canonicalize() syscall
    // on repeat requests — for a static server this is a ~50-100μs win
    // per request after the first access.
    let relative = normalized.trim_start_matches('/');
    if !relative.is_empty() {
        let candidate = state.canonical_root.join(relative);

        // Check cache first
        let resolved = if let Some(cached) = state.canonical_cache.get(&candidate) {
            Some(cached.clone())
        } else if let Ok(resolved) = candidate.canonicalize() {
            state
                .canonical_cache
                .insert(candidate.clone(), resolved.clone());
            Some(resolved)
        } else {
            None // File doesn't exist — let ServeDir handle 404
        };

        if let Some(resolved) = resolved {
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
    }

    Ok(())
}

/// Check for encoded traversal chars (%2e, %2f, %5c) case-insensitively
/// without allocating.  Replaces the previous `decoded.to_lowercase()`
/// which allocated a new String on every request.
#[inline]
fn has_encoded_traversal(s: &str) -> bool {
    let bytes = s.as_bytes();
    let len = bytes.len();
    if len < 3 {
        return false;
    }
    for i in 0..len - 2 {
        if bytes[i] == b'%' {
            let hi = bytes[i + 1];
            let lo = bytes[i + 2];
            // %2e / %2E / %2f / %2F / %5c / %5C
            if (hi == b'2' && (lo == b'e' || lo == b'E' || lo == b'f' || lo == b'F'))
                || (hi == b'5' && (lo == b'c' || lo == b'C'))
            {
                return true;
            }
        }
    }
    false
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
        assert_eq!(
            validate_path("/../etc/passwd", &state()),
            Err(StatusCode::FORBIDDEN)
        );
    }

    #[test]
    fn rejects_encoded_traversal() {
        assert_eq!(
            validate_path("/%2e%2e/etc/passwd", &state()),
            Err(StatusCode::FORBIDDEN)
        );
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
        assert_eq!(
            validate_path("/..\\etc\\passwd", &state()),
            Err(StatusCode::FORBIDDEN)
        );
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
        assert_eq!(
            validate_path("/file%00.txt", &state()),
            Err(StatusCode::BAD_REQUEST)
        );
    }

    #[test]
    fn rejects_null_byte_literal() {
        assert_eq!(
            validate_path("/file\0.txt", &state()),
            Err(StatusCode::BAD_REQUEST)
        );
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
        assert_eq!(
            validate_path("/CON.txt", &state()),
            Err(StatusCode::FORBIDDEN)
        );
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
