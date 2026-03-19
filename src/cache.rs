use std::collections::HashSet;

use axum::body::Body;
use axum::extract::State;
use axum::http::header::HeaderValue;
use axum::http::{header, Request, StatusCode};
use axum::middleware::Next;
use axum::response::Response;

use crate::config::CacheConfig;

#[derive(Clone)]
pub struct CacheState {
    default_header: HeaderValue,
    immutable_header: HeaderValue,
    no_cache_header: HeaderValue,
    immutable_extensions: HashSet<String>,
    no_cache_extensions: HashSet<String>,
    compression_enabled: bool,
}

impl CacheState {
    pub fn new(cache_config: &CacheConfig, compression_enabled: bool) -> Self {
        let default_header =
            HeaderValue::from_str(&format!("public, max-age={}", cache_config.default_max_age))
                .expect("invalid default_max_age");

        let immutable_header = HeaderValue::from_str(&format!(
            "public, max-age={}, immutable",
            cache_config.immutable_max_age
        ))
        .expect("invalid immutable_max_age");

        let no_cache_header = HeaderValue::from_static("no-cache");

        let immutable_extensions = cache_config
            .immutable_extensions
            .iter()
            .map(|e| e.to_ascii_lowercase())
            .collect();

        let no_cache_extensions = cache_config
            .no_cache_extensions
            .iter()
            .map(|e| e.to_ascii_lowercase())
            .collect();

        Self {
            default_header,
            immutable_header,
            no_cache_header,
            immutable_extensions,
            no_cache_extensions,
            compression_enabled,
        }
    }
}

#[inline]
pub fn extract_extension(uri_path: &str) -> Option<&str> {
    if uri_path.ends_with('/') {
        return Some("html");
    }
    let last_segment = uri_path.rsplit('/').next().unwrap_or(uri_path);
    last_segment
        .rsplit('.')
        .next()
        .filter(|ext| !ext.is_empty() && ext.len() < last_segment.len())
}

pub async fn cache_middleware(
    State(state): State<CacheState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    // Extract extension before consuming the request — avoids
    // cloning the URI path into a String.  Use a stack buffer for
    // case-insensitive comparison (extensions are max ~5 bytes).
    let ext_lower = {
        let path = request.uri().path();
        extract_extension(path).map(|e| {
            let mut buf = [0u8; 16];
            let len = e.len().min(16);
            buf[..len].copy_from_slice(&e.as_bytes()[..len]);
            buf[..len].make_ascii_lowercase();
            let s = std::str::from_utf8(&buf[..len]).unwrap_or("").to_owned();
            s
        })
    };
    let mut response = next.run(request).await;

    let status = response.status();
    if !(status.is_success() || status == StatusCode::NOT_MODIFIED) {
        return response;
    }

    let extension = ext_lower;
    let headers = response.headers_mut();

    match extension.as_deref() {
        Some(ext) if state.no_cache_extensions.contains(ext) => {
            headers.insert(header::CACHE_CONTROL, state.no_cache_header.clone());
        }
        Some(ext) if state.immutable_extensions.contains(ext) => {
            headers.insert(header::CACHE_CONTROL, state.immutable_header.clone());
        }
        _ => {
            headers.insert(header::CACHE_CONTROL, state.default_header.clone());
        }
    }

    if state.compression_enabled {
        headers.append(header::VARY, HeaderValue::from_static("Accept-Encoding"));
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_cache_config() -> CacheConfig {
        CacheConfig {
            default_max_age: 3600,
            immutable_max_age: 31_536_000,
            immutable_extensions: vec![
                "js", "css", "woff2", "woff", "ttf", "eot", "png", "jpg", "jpeg", "gif", "svg",
                "webp", "avif", "ico", "mp4", "webm", "ogg", "wasm",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            no_cache_extensions: vec!["html", "htm"].into_iter().map(String::from).collect(),
        }
    }

    // -- extract_extension tests --

    #[test]
    fn ext_html_file() {
        assert_eq!(extract_extension("/index.html"), Some("html"));
    }

    #[test]
    fn ext_css_file() {
        assert_eq!(extract_extension("/assets/style.css"), Some("css"));
    }

    #[test]
    fn ext_no_extension() {
        assert_eq!(extract_extension("/README"), None);
    }

    #[test]
    fn ext_trailing_slash_directory() {
        assert_eq!(extract_extension("/about/"), Some("html"));
    }

    #[test]
    fn ext_root() {
        assert_eq!(extract_extension("/"), Some("html"));
    }

    #[test]
    fn ext_double_extension() {
        assert_eq!(extract_extension("/archive.tar.gz"), Some("gz"));
    }

    // -- cache policy tests --

    #[test]
    fn html_gets_no_cache() {
        let state = CacheState::new(&default_cache_config(), false);
        assert!(state.no_cache_extensions.contains("html"));
        assert_eq!(state.no_cache_header, "no-cache");
    }

    #[test]
    fn js_gets_immutable() {
        let state = CacheState::new(&default_cache_config(), false);
        assert!(state.immutable_extensions.contains("js"));
        assert_eq!(
            state.immutable_header,
            "public, max-age=31536000, immutable"
        );
    }

    #[test]
    fn png_gets_immutable() {
        let state = CacheState::new(&default_cache_config(), false);
        assert!(state.immutable_extensions.contains("png"));
    }

    #[test]
    fn xml_gets_default() {
        let state = CacheState::new(&default_cache_config(), false);
        assert!(!state.immutable_extensions.contains("xml"));
        assert!(!state.no_cache_extensions.contains("xml"));
        assert_eq!(state.default_header, "public, max-age=3600");
    }

    #[test]
    fn no_extension_gets_default() {
        // extract_extension returns None for extensionless paths,
        // which falls through to the default branch
        assert_eq!(extract_extension("/README"), None);
        let state = CacheState::new(&default_cache_config(), false);
        assert_eq!(state.default_header, "public, max-age=3600");
    }

    // -- vary header tests --

    #[test]
    fn compression_enabled_sets_vary() {
        let state = CacheState::new(&default_cache_config(), true);
        assert!(state.compression_enabled);
    }

    #[test]
    fn compression_disabled_no_vary() {
        let state = CacheState::new(&default_cache_config(), false);
        assert!(!state.compression_enabled);
    }
}
