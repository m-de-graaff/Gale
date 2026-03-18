use std::time::Duration;

use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::Response;

use crate::config::LimitsConfig;
use crate::error::ErrorResponse;

#[derive(Clone)]
pub struct RequestLimitsState {
    max_body_size: u64,
    max_uri_length: usize,
    max_header_count: usize,
    max_header_size: usize,
    request_timeout: Option<Duration>,
}

impl RequestLimitsState {
    pub fn from_config(config: &LimitsConfig) -> Self {
        Self {
            max_body_size: config.max_body_size,
            max_uri_length: config.max_uri_length,
            max_header_count: config.max_header_count,
            max_header_size: config.max_header_size,
            request_timeout: if config.request_timeout_secs == 0 {
                None
            } else {
                Some(Duration::from_secs(config.request_timeout_secs))
            },
        }
    }
}

fn validate_request_limits(
    request: &Request<Body>,
    state: &RequestLimitsState,
) -> Result<(), StatusCode> {
    // Check URI length (cheapest check first)
    if state.max_uri_length > 0 {
        let uri_len = request.uri().to_string().len();
        if uri_len > state.max_uri_length {
            return Err(StatusCode::URI_TOO_LONG);
        }
    }

    // Check header count
    if state.max_header_count > 0 && request.headers().len() > state.max_header_count {
        return Err(StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE);
    }

    // Check individual header sizes
    if state.max_header_size > 0 {
        for (name, value) in request.headers() {
            let size = name.as_str().len() + value.len();
            if size > state.max_header_size {
                return Err(StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE);
            }
        }
    }

    // Check Content-Length for body size
    if state.max_body_size > 0 {
        if let Some(cl) = request.headers().get("content-length") {
            if let Ok(cl_str) = cl.to_str() {
                if let Ok(len) = cl_str.parse::<u64>() {
                    if len > state.max_body_size {
                        return Err(StatusCode::PAYLOAD_TOO_LARGE);
                    }
                }
            }
        }
    }

    Ok(())
}

pub async fn request_limits_middleware(
    State(state): State<RequestLimitsState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, ErrorResponse> {
    let uri = request.uri().to_string();

    validate_request_limits(&request, &state).map_err(|status| {
        tracing::warn!(status = status.as_u16(), %uri, "request rejected by limits");
        ErrorResponse::new(status)
    })?;

    match state.request_timeout {
        Some(duration) => tokio::time::timeout(duration, next.run(request))
            .await
            .map_err(|_| {
                tracing::warn!(status = 408, %uri, "request timed out");
                ErrorResponse::new(StatusCode::REQUEST_TIMEOUT)
            }),
        None => Ok(next.run(request).await),
    }
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderValue, Uri};

    use super::*;

    fn default_state() -> RequestLimitsState {
        RequestLimitsState {
            max_body_size: 10_485_760,
            max_uri_length: 8192,
            max_header_count: 100,
            max_header_size: 8192,
            request_timeout: Some(Duration::from_secs(30)),
        }
    }

    fn request_with_uri(uri: &str) -> Request<Body> {
        Request::builder()
            .uri(uri.parse::<Uri>().unwrap())
            .body(Body::empty())
            .unwrap()
    }

    // --- URI length ---

    #[test]
    fn uri_too_long_returns_414() {
        let state = RequestLimitsState {
            max_uri_length: 100,
            ..default_state()
        };
        let long_path = format!("/{}", "a".repeat(200));
        let req = request_with_uri(&long_path);
        assert_eq!(
            validate_request_limits(&req, &state),
            Err(StatusCode::URI_TOO_LONG)
        );
    }

    #[test]
    fn uri_within_limit_passes() {
        let state = default_state();
        let req = request_with_uri("/index.html");
        assert!(validate_request_limits(&req, &state).is_ok());
    }

    #[test]
    fn uri_at_exact_limit_passes() {
        let state = RequestLimitsState {
            max_uri_length: 11,
            ..default_state()
        };
        // "/index.html" is 11 chars
        let req = request_with_uri("/index.html");
        assert!(validate_request_limits(&req, &state).is_ok());
    }

    #[test]
    fn uri_limit_disabled_when_zero() {
        let state = RequestLimitsState {
            max_uri_length: 0,
            ..default_state()
        };
        // Use a URI longer than the default 8192 limit but within http::Uri's internal max
        let long_path = format!("/{}", "a".repeat(50_000));
        let req = request_with_uri(&long_path);
        assert!(validate_request_limits(&req, &state).is_ok());
    }

    // --- Header count ---

    #[test]
    fn too_many_headers_returns_431() {
        let state = RequestLimitsState {
            max_header_count: 2,
            ..default_state()
        };
        let req = Request::builder()
            .uri("/")
            .header("x-a", "1")
            .header("x-b", "2")
            .header("x-c", "3")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            validate_request_limits(&req, &state),
            Err(StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE)
        );
    }

    #[test]
    fn header_count_within_limit_passes() {
        let state = default_state();
        let req = Request::builder()
            .uri("/")
            .header("x-a", "1")
            .body(Body::empty())
            .unwrap();
        assert!(validate_request_limits(&req, &state).is_ok());
    }

    #[test]
    fn header_count_disabled_when_zero() {
        let state = RequestLimitsState {
            max_header_count: 0,
            ..default_state()
        };
        let mut builder = Request::builder().uri("/");
        for i in 0..200 {
            builder = builder.header(format!("x-header-{i}"), "value");
        }
        let req = builder.body(Body::empty()).unwrap();
        assert!(validate_request_limits(&req, &state).is_ok());
    }

    // --- Header size ---

    #[test]
    fn oversized_header_returns_431() {
        let state = RequestLimitsState {
            max_header_size: 50,
            ..default_state()
        };
        let big_value = "x".repeat(100);
        let req = Request::builder()
            .uri("/")
            .header("x-big", HeaderValue::from_str(&big_value).unwrap())
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            validate_request_limits(&req, &state),
            Err(StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE)
        );
    }

    #[test]
    fn header_size_within_limit_passes() {
        let state = default_state();
        let req = Request::builder()
            .uri("/")
            .header("x-small", "tiny")
            .body(Body::empty())
            .unwrap();
        assert!(validate_request_limits(&req, &state).is_ok());
    }

    #[test]
    fn header_size_disabled_when_zero() {
        let state = RequestLimitsState {
            max_header_size: 0,
            ..default_state()
        };
        let big_value = "x".repeat(100_000);
        let req = Request::builder()
            .uri("/")
            .header(
                "x-huge",
                HeaderValue::from_bytes(big_value.as_bytes()).unwrap(),
            )
            .body(Body::empty())
            .unwrap();
        assert!(validate_request_limits(&req, &state).is_ok());
    }

    // --- Body size (Content-Length) ---

    #[test]
    fn large_content_length_returns_413() {
        let state = default_state();
        let req = Request::builder()
            .uri("/")
            .header("content-length", "99999999")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            validate_request_limits(&req, &state),
            Err(StatusCode::PAYLOAD_TOO_LARGE)
        );
    }

    #[test]
    fn small_content_length_passes() {
        let state = default_state();
        let req = Request::builder()
            .uri("/")
            .header("content-length", "1024")
            .body(Body::empty())
            .unwrap();
        assert!(validate_request_limits(&req, &state).is_ok());
    }

    #[test]
    fn absent_content_length_passes() {
        let state = default_state();
        let req = request_with_uri("/");
        assert!(validate_request_limits(&req, &state).is_ok());
    }

    #[test]
    fn malformed_content_length_passes() {
        let state = default_state();
        let req = Request::builder()
            .uri("/")
            .header("content-length", "not-a-number")
            .body(Body::empty())
            .unwrap();
        assert!(validate_request_limits(&req, &state).is_ok());
    }

    #[test]
    fn body_size_disabled_when_zero() {
        let state = RequestLimitsState {
            max_body_size: 0,
            ..default_state()
        };
        let req = Request::builder()
            .uri("/")
            .header("content-length", "999999999999")
            .body(Body::empty())
            .unwrap();
        assert!(validate_request_limits(&req, &state).is_ok());
    }

    // --- All disabled ---

    #[test]
    fn all_limits_disabled_passes_everything() {
        let state = RequestLimitsState {
            max_body_size: 0,
            max_uri_length: 0,
            max_header_count: 0,
            max_header_size: 0,
            request_timeout: None,
        };
        let long_path = format!("/{}", "a".repeat(50_000));
        let big_value = "x".repeat(50_000);
        let req = Request::builder()
            .uri(long_path.parse::<Uri>().unwrap())
            .header("content-length", "999999999999")
            .header(
                "x-huge",
                HeaderValue::from_bytes(big_value.as_bytes()).unwrap(),
            )
            .body(Body::empty())
            .unwrap();
        assert!(validate_request_limits(&req, &state).is_ok());
    }

    // --- from_config ---

    #[test]
    fn from_config_timeout_zero_is_none() {
        let config = LimitsConfig {
            max_body_size: 0,
            max_uri_length: 0,
            max_header_count: 0,
            max_header_size: 0,
            request_timeout_secs: 0,
            read_timeout_secs: 0,
            write_timeout_secs: 0,
        };
        let state = RequestLimitsState::from_config(&config);
        assert!(state.request_timeout.is_none());
    }

    #[test]
    fn from_config_timeout_30_is_some() {
        let config = LimitsConfig {
            max_body_size: 10_485_760,
            max_uri_length: 8192,
            max_header_count: 100,
            max_header_size: 8192,
            request_timeout_secs: 30,
            read_timeout_secs: 10,
            write_timeout_secs: 10,
        };
        let state = RequestLimitsState::from_config(&config);
        assert_eq!(state.request_timeout, Some(Duration::from_secs(30)));
    }
}
