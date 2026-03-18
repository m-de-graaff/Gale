use axum::body::Body;
use axum::extract::State;
use axum::http::header::HeaderValue;
use axum::http::{header, Request};
use axum::middleware::Next;
use axum::response::Response;

use crate::config::SecurityHeadersConfig;

#[derive(Clone)]
pub struct SecurityHeadersState {
    csp: Option<HeaderValue>,
    hsts: Option<HeaderValue>,
    x_content_type_options: Option<HeaderValue>,
    x_frame_options: Option<HeaderValue>,
    x_xss_protection: HeaderValue,
    referrer_policy: Option<HeaderValue>,
    permissions_policy: Option<HeaderValue>,
    server: Option<HeaderValue>,
}

fn non_empty_header(value: &str) -> Option<HeaderValue> {
    if value.is_empty() {
        None
    } else {
        HeaderValue::from_str(value).ok()
    }
}

impl SecurityHeadersState {
    pub fn from_config(config: &SecurityHeadersConfig) -> Self {
        let hsts = if config.hsts_max_age == 0 {
            None
        } else if config.hsts_include_subdomains {
            HeaderValue::from_str(&format!(
                "max-age={}; includeSubDomains",
                config.hsts_max_age
            ))
            .ok()
        } else {
            HeaderValue::from_str(&format!("max-age={}", config.hsts_max_age)).ok()
        };

        let x_content_type_options = if config.x_content_type_options {
            Some(HeaderValue::from_static("nosniff"))
        } else {
            None
        };

        SecurityHeadersState {
            csp: non_empty_header(&config.csp),
            hsts,
            x_content_type_options,
            x_frame_options: non_empty_header(&config.x_frame_options),
            x_xss_protection: HeaderValue::from_static("0"),
            referrer_policy: non_empty_header(&config.referrer_policy),
            permissions_policy: non_empty_header(&config.permissions_policy),
            server: non_empty_header(&config.server_header),
        }
    }
}

pub async fn security_headers_middleware(
    State(state): State<SecurityHeadersState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    if let Some(ref val) = state.csp {
        headers.insert(header::CONTENT_SECURITY_POLICY, val.clone());
    }
    if let Some(ref val) = state.hsts {
        headers.insert(header::STRICT_TRANSPORT_SECURITY, val.clone());
    }
    if let Some(ref val) = state.x_content_type_options {
        headers.insert(header::X_CONTENT_TYPE_OPTIONS, val.clone());
    }
    if let Some(ref val) = state.x_frame_options {
        headers.insert(header::X_FRAME_OPTIONS, val.clone());
    }
    headers.insert(
        header::HeaderName::from_static("x-xss-protection"),
        state.x_xss_protection.clone(),
    );
    if let Some(ref val) = state.referrer_policy {
        headers.insert(header::REFERRER_POLICY, val.clone());
    }
    if let Some(ref val) = state.permissions_policy {
        headers.insert(
            header::HeaderName::from_static("permissions-policy"),
            val.clone(),
        );
    }
    if let Some(ref val) = state.server {
        headers.insert(header::SERVER, val.clone());
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> SecurityHeadersConfig {
        SecurityHeadersConfig {
            csp: "default-src 'self'".to_string(),
            hsts_max_age: 31_536_000,
            hsts_include_subdomains: true,
            x_content_type_options: true,
            x_frame_options: "DENY".to_string(),
            referrer_policy: "strict-origin-when-cross-origin".to_string(),
            permissions_policy: "camera=(), microphone=(), geolocation=()".to_string(),
            server_header: String::new(),
        }
    }

    #[test]
    fn default_config_produces_all_headers() {
        let state = SecurityHeadersState::from_config(&default_config());
        assert!(state.csp.is_some());
        assert!(state.hsts.is_some());
        assert!(state.x_content_type_options.is_some());
        assert!(state.x_frame_options.is_some());
        assert!(state.referrer_policy.is_some());
        assert!(state.permissions_policy.is_some());
        // Server header is empty by default
        assert!(state.server.is_none());
    }

    #[test]
    fn default_header_values() {
        let state = SecurityHeadersState::from_config(&default_config());
        assert_eq!(state.csp.unwrap(), "default-src 'self'");
        assert_eq!(state.hsts.unwrap(), "max-age=31536000; includeSubDomains");
        assert_eq!(state.x_content_type_options.unwrap(), "nosniff");
        assert_eq!(state.x_frame_options.unwrap(), "DENY");
        assert_eq!(
            state.referrer_policy.unwrap(),
            "strict-origin-when-cross-origin"
        );
        assert_eq!(
            state.permissions_policy.unwrap(),
            "camera=(), microphone=(), geolocation=()"
        );
    }

    #[test]
    fn hsts_without_include_subdomains() {
        let mut cfg = default_config();
        cfg.hsts_include_subdomains = false;
        let state = SecurityHeadersState::from_config(&cfg);
        assert_eq!(state.hsts.unwrap(), "max-age=31536000");
    }

    #[test]
    fn hsts_max_age_zero_disables() {
        let mut cfg = default_config();
        cfg.hsts_max_age = 0;
        let state = SecurityHeadersState::from_config(&cfg);
        assert!(state.hsts.is_none());
    }

    #[test]
    fn empty_csp_disables() {
        let mut cfg = default_config();
        cfg.csp = String::new();
        let state = SecurityHeadersState::from_config(&cfg);
        assert!(state.csp.is_none());
    }

    #[test]
    fn empty_x_frame_options_disables() {
        let mut cfg = default_config();
        cfg.x_frame_options = String::new();
        let state = SecurityHeadersState::from_config(&cfg);
        assert!(state.x_frame_options.is_none());
    }

    #[test]
    fn empty_referrer_policy_disables() {
        let mut cfg = default_config();
        cfg.referrer_policy = String::new();
        let state = SecurityHeadersState::from_config(&cfg);
        assert!(state.referrer_policy.is_none());
    }

    #[test]
    fn empty_permissions_policy_disables() {
        let mut cfg = default_config();
        cfg.permissions_policy = String::new();
        let state = SecurityHeadersState::from_config(&cfg);
        assert!(state.permissions_policy.is_none());
    }

    #[test]
    fn x_content_type_options_false_disables() {
        let mut cfg = default_config();
        cfg.x_content_type_options = false;
        let state = SecurityHeadersState::from_config(&cfg);
        assert!(state.x_content_type_options.is_none());
    }

    #[test]
    fn x_xss_protection_always_zero() {
        let state = SecurityHeadersState::from_config(&default_config());
        assert_eq!(state.x_xss_protection, "0");
    }

    #[test]
    fn server_header_present_when_configured() {
        let mut cfg = default_config();
        cfg.server_header = "Gale/1.0".to_string();
        let state = SecurityHeadersState::from_config(&cfg);
        assert_eq!(state.server.unwrap(), "Gale/1.0");
    }

    #[test]
    fn invalid_header_value_handled_gracefully() {
        let mut cfg = default_config();
        // Header values cannot contain non-visible ASCII (except tab)
        cfg.csp = "invalid\x01value".to_string();
        let state = SecurityHeadersState::from_config(&cfg);
        // non_empty_header returns None for invalid values via HeaderValue::from_str().ok()
        assert!(state.csp.is_none());
    }
}
