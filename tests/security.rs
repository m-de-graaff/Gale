//! Security integration tests for Gale.
//!
//! Tests the full middleware stack (minus rate_limit and logging which require
//! ConnectInfo<SocketAddr> from a real TCP listener) using tower::ServiceExt::oneshot().
//!
//! Covers:
//! - Path traversal fuzzing (~90 vectors)
//! - Header injection / request limits
//! - Security header verification on all response types
//! - Windows-specific attack vectors (conditional)
//!
//! # Manual scanning guide
//!
//! ```bash
//! # securityheaders.com — start Gale then visit:
//! # https://securityheaders.com/?q=http://your-ip:8080
//!
//! # nikto
//! nikto -h http://localhost:8080 -Tuning x -output nikto-report.html -Format htm
//!
//! # OWASP ZAP
//! docker run -t ghcr.io/zaproxy/zaproxy:stable zap-baseline.py -t http://host.docker.internal:8080
//!
//! # curl spot-checks
//! curl -sI http://localhost:8080/ | grep -iE "^(content-security|strict-transport|x-content-type|x-frame|x-xss|referrer-policy|permissions-policy|server):"
//! curl -v http://localhost:8080/../etc/passwd        # expect 403
//! curl -v http://localhost:8080/.env                 # expect 403
//! curl -v http://localhost:8080/%2e%2e/etc/passwd    # expect 403
//! ```

use std::path::PathBuf;

use axum::body::Body;
use axum::http::{HeaderValue, Request, Response, StatusCode};
use axum::middleware;
use axum::Router;
use tower::ServiceExt;

use gale_lib::cache::{self, CacheState};
use gale_lib::compression;
use gale_lib::config::Config;
use gale_lib::security::headers::{SecurityHeadersState, security_headers_middleware};
use gale_lib::security::limits::{RequestLimitsState, request_limits_middleware};
use gale_lib::security::path::{PathSecurityState, path_security_middleware};
use gale_lib::static_files;

// ---------------------------------------------------------------------------
// Test infrastructure
// ---------------------------------------------------------------------------

fn build_test_app() -> Router {
    let config = Config::defaults();

    let canonical_root = PathBuf::from(&config.root)
        .canonicalize()
        .expect("test public/ directory must exist — run from repo root");

    let security_state = PathSecurityState {
        canonical_root,
        block_dotfiles: true,
    };
    let headers_state = SecurityHeadersState::from_config(&config.security_headers);
    let limits_state = RequestLimitsState::from_config(&config.limits);
    let compression_layer = compression::build_layer(&config.compression);
    let cache_state = CacheState::new(&config.cache, config.compression.enabled);

    // Mirrors server.rs middleware stack order (minus rate_limit + logging)
    static_files::create_static_router(&config)
        .layer(middleware::from_fn_with_state(
            cache_state,
            cache::cache_middleware,
        ))
        .layer(compression_layer)
        .layer(middleware::from_fn_with_state(
            security_state,
            path_security_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            limits_state,
            request_limits_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            headers_state,
            security_headers_middleware,
        ))
}

async fn get(app: &Router, uri: &str) -> Response<Body> {
    app.clone()
        .oneshot(
            Request::builder()
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

/// Assert status is one of the expected codes; returns error message on mismatch.
fn assert_blocked(uri: &str, status: StatusCode, expected: &[StatusCode]) -> Option<String> {
    if expected.contains(&status) {
        None
    } else {
        Some(format!(
            "  {uri}  →  {status}  (expected one of {expected:?})"
        ))
    }
}

// ---------------------------------------------------------------------------
// 5b. Path traversal fuzzing
// ---------------------------------------------------------------------------

#[tokio::test]
async fn path_traversal_fuzzing() {
    let app = build_test_app();
    let mut failures: Vec<String> = Vec::new();

    // Each entry: (uri, list of acceptable status codes)
    let blocked_403 = &[StatusCode::FORBIDDEN][..];
    let blocked_400 = &[StatusCode::BAD_REQUEST][..];
    let blocked_400_or_403 = &[StatusCode::BAD_REQUEST, StatusCode::FORBIDDEN][..];

    let vectors: Vec<(&str, &[StatusCode])> = vec![
        // --- Basic ".." traversal ---
        ("/../etc/passwd", blocked_403),
        ("/../../etc/passwd", blocked_403),
        ("/../../../etc/passwd", blocked_403),
        ("/../../../../etc/passwd", blocked_403),
        ("/../../../../../etc/passwd", blocked_403),
        ("/../Cargo.toml", blocked_403),
        ("/./../../etc/passwd", blocked_403),
        ("/foo/../../../etc/passwd", blocked_403),
        ("/static/../../../etc/passwd", blocked_403),
        ("/a/b/c/../../../../etc/passwd", blocked_403),
        ("/../../../../../../../etc/shadow", blocked_403),
        ("/..%2f..%2f..%2fetc/passwd", blocked_400_or_403),
        // --- URL-encoded ".." (%2e%2e) ---
        ("/%2e%2e/etc/passwd", blocked_403),
        ("/%2E%2E/etc/passwd", blocked_403),
        ("/%2e%2e/%2e%2e/etc/passwd", blocked_403),
        ("/%2e%2e%2f%2e%2e%2fetc%2fpasswd", blocked_403),
        ("/foo/%2e%2e/%2e%2e/etc/passwd", blocked_403),
        ("/%2e%2e/Cargo.toml", blocked_403),
        ("/%2E%2E/%2E%2E/etc/passwd", blocked_403),
        // --- Double-encoded (%252e%252e) ---
        ("/%252e%252e/Cargo.toml", blocked_400),
        ("/%252e%252e/%252e%252e/etc/passwd", blocked_400),
        ("/%252e%252e%252f%252e%252e%252fetc/passwd", blocked_400),
        ("/%252E%252E/etc/passwd", blocked_400),
        // Triple-encoded: only double-encoding is rejected; triple is let through
        // and resolves to a literal path that doesn't exist. Not a security gap.
        // --- Backslash traversal ---
        ("/..\\etc\\passwd", blocked_403),
        ("/..\\..\\etc\\passwd", blocked_403),
        ("/..\\..\\..\\etc\\passwd", blocked_403),
        ("/foo\\..\\..\\etc\\passwd", blocked_403),
        ("/..\\..\\Cargo.toml", blocked_403),
        // --- Encoded backslash (%5c) ---
        ("/%2e%2e%5cetc%5cpasswd", blocked_403),
        ("/%2e%2e%5C%2e%2e%5Cetc%5Cpasswd", blocked_403),
        ("/..%5c..%5cetc%5cpasswd", blocked_400_or_403),
        // --- Null byte injection ---
        ("/file%00.txt", blocked_400),
        ("/%00", blocked_400),
        ("/index.html%00.bak", blocked_400),
        ("/etc/passwd%00.html", blocked_400),
        ("/..%00/etc/passwd", blocked_400),
        ("/%2500", blocked_400),
        ("/%00%2e%2e/etc/passwd", blocked_400),
        // --- Mixed / combined attacks ---
        ("/%2e%2e%00/etc/passwd", blocked_400),
        ("/%252e%252e%00/Cargo.toml", blocked_400),
        ("/%2e%2e/%252e%252e/etc/passwd", blocked_400),
        ("/%c0%ae%c0%ae/etc/passwd", blocked_400_or_403),
        // --- Unicode overlong encoding ---
        ("/%c0%ae%c0%ae/", blocked_400_or_403),
        ("/%e0%80%ae%e0%80%ae/", blocked_400_or_403),
        ("/%c0%af../etc/passwd", blocked_400_or_403),
        ("/%c1%9c../etc/passwd", blocked_400_or_403),
        // --- Dotfile blocking ---
        ("/.env", blocked_403),
        ("/.git/HEAD", blocked_403),
        ("/.git/config", blocked_403),
        ("/.aws/credentials", blocked_403),
        ("/.ssh/id_rsa", blocked_403),
        ("/.htaccess", blocked_403),
        ("/.htpasswd", blocked_403),
        ("/.DS_Store", blocked_403),
        ("/assets/.hidden", blocked_403),
        ("/.dockerenv", blocked_403),
        // --- Encoded dotfiles ---
        ("/%2egit/config", blocked_403),
        ("/%2eenv", blocked_403),
    ];

    for (uri, expected) in &vectors {
        let resp = get(&app, uri).await;
        if let Some(msg) = assert_blocked(uri, resp.status(), expected) {
            failures.push(msg);
        }
    }

    if !failures.is_empty() {
        panic!(
            "\n{} path traversal vector(s) failed:\n{}\n",
            failures.len(),
            failures.join("\n")
        );
    }
}

#[tokio::test]
async fn legitimate_paths_not_blocked() {
    let app = build_test_app();
    let ok_or_not_found = &[StatusCode::OK, StatusCode::NOT_FOUND];

    let vectors: Vec<(&str, &[StatusCode])> = vec![
        ("/", &[StatusCode::OK]),
        ("/index.html", &[StatusCode::OK]),
        ("/file..name.txt", ok_or_not_found),
        // "/....//test" starts with "." so it's correctly blocked as a dotfile
        ("/path/to/file.txt", ok_or_not_found),
        ("/hello%20world.html", ok_or_not_found),
        ("/normal-path", ok_or_not_found),
    ];

    let mut failures: Vec<String> = Vec::new();
    for (uri, expected) in &vectors {
        let resp = get(&app, uri).await;
        if !expected.contains(&resp.status()) {
            failures.push(format!(
                "  {uri}  →  {}  (expected one of {expected:?})",
                resp.status()
            ));
        }
    }

    if !failures.is_empty() {
        panic!(
            "\n{} legitimate path(s) incorrectly blocked:\n{}\n",
            failures.len(),
            failures.join("\n")
        );
    }
}

// ---------------------------------------------------------------------------
// 5c. Header injection / request limits
// ---------------------------------------------------------------------------

#[tokio::test]
async fn oversized_header_rejected() {
    let app = build_test_app();
    let big_value = "x".repeat(10_000);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/")
                .header("x-attack", HeaderValue::from_bytes(big_value.as_bytes()).unwrap())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE);
}

#[tokio::test]
async fn too_many_headers_rejected() {
    let app = build_test_app();
    let mut builder = Request::builder().uri("/");
    for i in 0..101 {
        builder = builder.header(format!("x-hdr-{i}"), "val");
    }
    let resp = app
        .oneshot(builder.body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE);
}

#[tokio::test]
async fn oversized_uri_rejected() {
    let app = build_test_app();
    let long_uri = format!("/{}", "a".repeat(9000));
    let resp = app
        .oneshot(
            Request::builder()
                .uri(long_uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::URI_TOO_LONG);
}

#[tokio::test]
async fn large_content_length_rejected() {
    let app = build_test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/")
                .header("content-length", "999999999")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

// Note: True CRLF injection (\r\n in headers) is blocked at the HTTP parser level
// by hyper/http crate — HeaderValue::from_str() and from_bytes() reject control
// characters at construction time. This is not a gap in Gale's security.

// ---------------------------------------------------------------------------
// 5d. Security header verification
// ---------------------------------------------------------------------------

/// Check that all expected OWASP security headers are present on a response.
fn verify_security_headers(resp: &Response<Body>, context: &str) {
    let h = resp.headers();

    assert!(
        h.get("content-security-policy").is_some(),
        "{context}: missing Content-Security-Policy"
    );
    assert!(
        h.get("strict-transport-security").is_some(),
        "{context}: missing Strict-Transport-Security"
    );
    assert_eq!(
        h.get("x-content-type-options").map(|v| v.as_bytes()),
        Some(b"nosniff".as_slice()),
        "{context}: X-Content-Type-Options should be 'nosniff'"
    );
    assert!(
        h.get("x-frame-options").is_some(),
        "{context}: missing X-Frame-Options"
    );
    assert_eq!(
        h.get("x-xss-protection").map(|v| v.as_bytes()),
        Some(b"0".as_slice()),
        "{context}: X-XSS-Protection should be '0'"
    );
    assert!(
        h.get("referrer-policy").is_some(),
        "{context}: missing Referrer-Policy"
    );
    assert!(
        h.get("permissions-policy").is_some(),
        "{context}: missing Permissions-Policy"
    );
    assert!(
        h.get("server").is_none(),
        "{context}: Server header should be absent"
    );
}

#[tokio::test]
async fn security_headers_on_200() {
    let app = build_test_app();
    let resp = get(&app, "/").await;
    assert_eq!(resp.status(), StatusCode::OK);
    verify_security_headers(&resp, "200 OK");
}

#[tokio::test]
async fn security_headers_on_404() {
    let app = build_test_app();
    let resp = get(&app, "/nonexistent-page.html").await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    verify_security_headers(&resp, "404 Not Found");
}

#[tokio::test]
async fn security_headers_on_403() {
    let app = build_test_app();
    let resp = get(&app, "/.env").await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    verify_security_headers(&resp, "403 Forbidden");
}

#[tokio::test]
async fn security_headers_on_400() {
    let app = build_test_app();
    let resp = get(&app, "/file%00.txt").await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    verify_security_headers(&resp, "400 Bad Request");
}

#[tokio::test]
async fn security_headers_on_414() {
    let app = build_test_app();
    let long_uri = format!("/{}", "a".repeat(9000));
    let resp = get(&app, &long_uri).await;
    assert_eq!(resp.status(), StatusCode::URI_TOO_LONG);
    verify_security_headers(&resp, "414 URI Too Long");
}

// ---------------------------------------------------------------------------
// 5e. Windows-specific tests
// ---------------------------------------------------------------------------

#[cfg(windows)]
mod windows {
    use super::*;

    #[tokio::test]
    async fn ads_blocked() {
        let app = build_test_app();
        let vectors = vec![
            "/file.txt:hidden",
            "/file.txt:$DATA",
            "/index.html:secret",
        ];
        for uri in vectors {
            let resp = get(&app, uri).await;
            assert_eq!(
                resp.status(),
                StatusCode::FORBIDDEN,
                "ADS vector not blocked: {uri}"
            );
        }
    }

    #[tokio::test]
    async fn reserved_device_names_blocked() {
        let app = build_test_app();
        let devices = vec![
            "/CON", "/PRN", "/NUL", "/AUX",
            "/COM1", "/COM2", "/COM3", "/COM4", "/COM5",
            "/COM6", "/COM7", "/COM8", "/COM9",
            "/LPT1", "/LPT2", "/LPT3", "/LPT4", "/LPT5",
            "/LPT6", "/LPT7", "/LPT8", "/LPT9",
            // With extensions
            "/CON.txt", "/NUL.html", "/COM1.log",
            // Lowercase
            "/con", "/nul", "/aux", "/prn",
        ];
        let mut failures = Vec::new();
        for uri in devices {
            let resp = get(&app, uri).await;
            if resp.status() != StatusCode::FORBIDDEN {
                failures.push(format!("  {uri}  →  {}", resp.status()));
            }
        }
        if !failures.is_empty() {
            panic!(
                "\n{} Windows device name(s) not blocked:\n{}\n",
                failures.len(),
                failures.join("\n")
            );
        }
    }

    #[tokio::test]
    async fn unc_paths_blocked() {
        let app = build_test_app();
        let resp = get(&app, "//server/share").await;
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "UNC path not blocked"
        );
    }

    #[tokio::test]
    async fn windows_security_headers_on_blocked() {
        let app = build_test_app();
        // ADS attack should still get security headers
        let resp = get(&app, "/file.txt:hidden").await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        verify_security_headers(&resp, "Windows ADS 403");
    }
}
