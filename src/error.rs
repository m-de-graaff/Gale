use std::convert::Infallible;
use std::future::{ready, Ready};
use std::path::Path;
use std::task::{Context, Poll};

use axum::body::{Body, Bytes};
use axum::http::header::{CONTENT_LENGTH, CONTENT_TYPE};
use axum::http::{Request, Response, StatusCode};
use axum::response::IntoResponse;
use tower_service::Service;

use crate::mime_types;

/// Generates a styled HTML error page for any HTTP status code.
pub fn error_html(status: StatusCode) -> String {
    let code = status.as_u16();
    let reason = status.canonical_reason().unwrap_or("Error");
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{code} — {reason}</title>
<style>
  body {{ font-family: system-ui, -apple-system, sans-serif; display: flex;
         justify-content: center; align-items: center; min-height: 100vh;
         margin: 0; background: #fafafa; color: #333; }}
  .container {{ text-align: center; }}
  h1 {{ font-size: 4rem; margin: 0; color: #888; }}
  p  {{ font-size: 1.2rem; color: #666; }}
</style>
</head>
<body>
<div class="container">
  <h1>{code}</h1>
  <p>{reason}</p>
</div>
</body>
</html>"#
    )
}

/// An error response that renders a styled HTML error page.
///
/// Used by middleware to return error responses with proper HTML bodies
/// instead of bare status codes with empty bodies.
pub struct ErrorResponse {
    status: StatusCode,
}

impl ErrorResponse {
    pub fn new(status: StatusCode) -> Self {
        Self { status }
    }
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> Response<Body> {
        let html = error_html(self.status);
        Response::builder()
            .status(self.status)
            .header(CONTENT_TYPE, "text/html; charset=utf-8")
            .header(CONTENT_LENGTH, html.len().to_string())
            .header("x-content-type-options", "nosniff")
            .body(Body::from(html))
            .unwrap()
    }
}

#[derive(Clone)]
pub struct NotFoundService {
    body: Bytes,
    content_type: &'static str,
}

impl NotFoundService {
    pub fn from_config(error_page_404: &str) -> Self {
        if error_page_404.is_empty() {
            let html = error_html(StatusCode::NOT_FOUND);
            return Self {
                body: Bytes::from(html),
                content_type: "text/html; charset=utf-8",
            };
        }

        match std::fs::read(error_page_404) {
            Ok(contents) => {
                let content_type = Path::new(error_page_404)
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(mime_types::from_extension)
                    .unwrap_or("text/html; charset=utf-8");

                Self {
                    body: Bytes::from(contents),
                    content_type,
                }
            }
            Err(e) => {
                tracing::warn!(
                    path = error_page_404,
                    error = %e,
                    "failed to read custom 404 page, using built-in"
                );
                let html = error_html(StatusCode::NOT_FOUND);
                Self {
                    body: Bytes::from(html),
                    content_type: "text/html; charset=utf-8",
                }
            }
        }
    }
}

impl<B> Service<Request<B>> for NotFoundService {
    type Response = Response<Body>;
    type Error = Infallible;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: Request<B>) -> Self::Future {
        let response = Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(CONTENT_TYPE, self.content_type)
            .header(CONTENT_LENGTH, self.body.len().to_string())
            .body(Body::from(self.body.clone()))
            .unwrap();

        ready(Ok(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_html_contains_status_code() {
        let html = error_html(StatusCode::NOT_FOUND);
        assert!(html.contains("404"));
    }

    #[test]
    fn error_html_contains_reason_phrase() {
        let html = error_html(StatusCode::NOT_FOUND);
        assert!(html.contains("Not Found"));
    }

    #[test]
    fn error_html_different_status_codes() {
        let html = error_html(StatusCode::FORBIDDEN);
        assert!(html.contains("403"));
        assert!(html.contains("Forbidden"));

        let html = error_html(StatusCode::URI_TOO_LONG);
        assert!(html.contains("414"));
        assert!(html.contains("URI Too Long"));
    }

    #[test]
    fn error_response_sets_correct_status() {
        let resp = ErrorResponse::new(StatusCode::FORBIDDEN).into_response();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn error_response_sets_content_type_and_nosniff() {
        let resp = ErrorResponse::new(StatusCode::BAD_REQUEST).into_response();
        assert_eq!(
            resp.headers().get(CONTENT_TYPE).unwrap(),
            "text/html; charset=utf-8"
        );
        assert_eq!(
            resp.headers().get("x-content-type-options").unwrap(),
            "nosniff"
        );
    }

    #[test]
    fn error_response_sets_content_length() {
        let status = StatusCode::PAYLOAD_TOO_LARGE;
        let expected_len = error_html(status).len();
        let resp = ErrorResponse::new(status).into_response();
        let cl: usize = resp
            .headers()
            .get(CONTENT_LENGTH)
            .unwrap()
            .to_str()
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(cl, expected_len);
    }

    #[test]
    fn not_found_service_returns_404_status() {
        let mut svc = NotFoundService::from_config("");
        let req = Request::builder().body(Body::empty()).unwrap();
        let resp = svc.call(req).into_inner().unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
