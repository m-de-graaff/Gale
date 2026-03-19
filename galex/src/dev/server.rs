//! Dev proxy server with WebSocket broadcast hub.
//!
//! Proxies all HTTP requests to the generated backend server while:
//! - Injecting the dev client script into HTML responses
//! - Hosting a WebSocket endpoint for browser reload notifications
//! - Serving the error overlay JS/CSS assets

use std::net::SocketAddr;

use axum::body::Body;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::http::{Request, Response, StatusCode};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use futures_util::stream::StreamExt;
use futures_util::SinkExt;
use tokio::sync::broadcast;

/// Messages sent from the dev server to connected browsers.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum DevMessage {
    /// Full page reload (server-side code changed).
    Reload,
    /// CSS-only reload (no page reload, just re-fetch stylesheets).
    CssReload,
    /// A specific static asset changed.
    AssetReload { path: String },
    /// Compilation errors — show error overlay.
    Error { errors: Vec<DevError> },
    /// Errors cleared — hide overlay.
    ErrorCleared,
}

/// A structured compilation error for the browser overlay.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DevError {
    /// Source file path.
    pub file: String,
    /// Line number (1-indexed).
    pub line: u32,
    /// Column number (1-indexed).
    pub col: u32,
    /// Error message.
    pub message: String,
    /// Error code (e.g. "GX0001"), if available.
    pub code: Option<String>,
    /// The source line containing the error.
    pub source_line: Option<String>,
    /// Suggestion for fixing the error, if available.
    pub suggestion: Option<String>,
}

/// Shared state for the dev server.
#[derive(Clone)]
pub struct DevServerState {
    /// Broadcast sender for pushing messages to all browsers.
    pub tx: broadcast::Sender<DevMessage>,
    /// Port where the generated backend server is running.
    pub backend_port: u16,
}

/// Start the dev proxy server.
///
/// This runs an Axum server that:
/// 1. Serves `/__gale_dev/ws` — WebSocket for reload notifications
/// 2. Serves `/__gale_dev/overlay.js` — the dev client script
/// 3. Serves `/__gale_dev/overlay.css` — error overlay styles
/// 4. Proxies everything else to `localhost:{backend_port}`
pub async fn run_dev_server(port: u16, backend_port: u16, tx: broadcast::Sender<DevMessage>) {
    let state = DevServerState { tx, backend_port };

    let app = Router::new()
        .route("/__gale_dev/ws", get(ws_handler))
        .route("/__gale_dev/overlay.js", get(overlay_js))
        .route("/__gale_dev/overlay.css", get(overlay_css))
        .fallback(proxy_handler)
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

/// WebSocket handler — subscribes to broadcast and forwards messages.
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<DevServerState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, state.tx))
}

async fn handle_ws(socket: WebSocket, tx: broadcast::Sender<DevMessage>) {
    let mut rx = tx.subscribe();
    let (mut sender, mut _receiver) = socket.split();

    // Forward broadcast messages to the WebSocket client
    while let Ok(msg) = rx.recv().await {
        let json = match serde_json::to_string(&msg) {
            Ok(j) => j,
            Err(_) => continue,
        };
        if sender.send(Message::Text(json.into())).await.is_err() {
            break; // Client disconnected
        }
    }
}

/// Serve the dev overlay JavaScript (embedded at compile time).
async fn overlay_js() -> impl IntoResponse {
    (
        [("content-type", "application/javascript; charset=utf-8")],
        include_str!("overlay.js"),
    )
}

/// Serve the dev overlay CSS (embedded at compile time).
async fn overlay_css() -> impl IntoResponse {
    (
        [("content-type", "text/css; charset=utf-8")],
        include_str!("overlay.css"),
    )
}

/// Reverse proxy — forward requests to the backend server.
///
/// Uses a simple TCP-level approach: connect to backend, send the request,
/// read the response. Injects the dev client script into HTML responses.
async fn proxy_handler(State(state): State<DevServerState>, req: Request<Body>) -> Response<Body> {
    use axum::http::uri::Uri;

    // Build the backend URI
    let path_and_query = req
        .uri()
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");
    let backend_uri: Uri = format!("http://127.0.0.1:{}{}", state.backend_port, path_and_query)
        .parse()
        .unwrap();

    // Connect to backend via TCP and send the request using hyper
    let stream =
        match tokio::net::TcpStream::connect(format!("127.0.0.1:{}", state.backend_port)).await {
            Ok(s) => s,
            Err(_) => {
                return Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .header("content-type", "text/html; charset=utf-8")
                    .body(Body::from(backend_down_html()))
                    .unwrap();
            }
        };

    // Use hyper's client connection
    let io = hyper_util::rt::TokioIo::new(stream);
    let (mut sender, conn) = match hyper::client::conn::http1::handshake(io).await {
        Ok(parts) => parts,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .header("content-type", "text/html; charset=utf-8")
                .body(Body::from(backend_down_html()))
                .unwrap();
        }
    };

    // Drive the connection in the background
    tokio::spawn(async move {
        let _ = conn.await;
    });

    // Build the proxied request with the backend URI
    let (mut parts, body) = req.into_parts();
    parts.uri = backend_uri;
    let proxy_req = Request::from_parts(parts, body);

    // Send the request
    let response = match sender.send_request(proxy_req).await {
        Ok(resp) => resp,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .header("content-type", "text/html; charset=utf-8")
                .body(Body::from(backend_down_html()))
                .unwrap();
        }
    };

    // Collect the full response body
    use http_body_util::BodyExt;
    let (mut parts, incoming) = response.into_parts();
    let collected = incoming.collect().await.unwrap_or_default();
    let body_bytes = collected.to_bytes();

    // Remove stale content-length / transfer-encoding — Axum will set
    // the correct values based on the new body we construct below.
    // Without this, the browser receives a Content-Length that doesn't
    // match the actual body (especially after dev script injection),
    // causing ERR_EMPTY_RESPONSE.
    parts.headers.remove("content-length");
    parts.headers.remove("transfer-encoding");

    let is_html = parts
        .headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|ct| ct.contains("text/html"))
        .unwrap_or(false);

    if is_html {
        let html = String::from_utf8_lossy(&body_bytes);
        let injected = inject_dev_script(&html);
        Response::from_parts(parts, Body::from(injected))
    } else {
        Response::from_parts(parts, Body::from(body_bytes))
    }
}

/// Inject the dev client script tag before `</body>`.
fn inject_dev_script(html: &str) -> String {
    const SCRIPT_TAG: &str = r#"<link rel="stylesheet" href="/__gale_dev/overlay.css"><script src="/__gale_dev/overlay.js"></script>"#;

    if let Some(pos) = html.rfind("</body>") {
        let mut result = String::with_capacity(html.len() + SCRIPT_TAG.len());
        result.push_str(&html[..pos]);
        result.push_str(SCRIPT_TAG);
        result.push_str(&html[pos..]);
        result
    } else {
        // No </body> found — append to end
        format!("{html}{SCRIPT_TAG}")
    }
}

/// HTML shown when the backend server is not running.
fn backend_down_html() -> String {
    r#"<!DOCTYPE html>
<html><head><title>Gale Dev — Building...</title>
<link rel="stylesheet" href="/__gale_dev/overlay.css">
</head><body>
<div id="gale-dev-overlay" style="display:flex;align-items:center;justify-content:center">
<div class="gale-error-card" style="text-align:center">
<h2 style="margin:0 0 1rem">Building...</h2>
<p style="color:#888">The server is starting up. This page will reload automatically.</p>
</div></div>
<script src="/__gale_dev/overlay.js"></script>
</body></html>"#
        .to_string()
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inject_script_before_body_close() {
        let html = "<html><body><h1>Hello</h1></body></html>";
        let result = inject_dev_script(html);
        assert!(result.contains("/__gale_dev/overlay.js"));
        assert!(result.contains("</body>"));
        // Script should appear before </body>
        let script_pos = result.find("overlay.js").unwrap();
        let body_pos = result.find("</body>").unwrap();
        assert!(script_pos < body_pos);
    }

    #[test]
    fn inject_script_no_body_tag() {
        let html = "<html><h1>No body</h1></html>";
        let result = inject_dev_script(html);
        assert!(result.contains("overlay.js"));
    }

    #[test]
    fn dev_message_serializes() {
        let msg = DevMessage::Reload;
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"Reload\""));

        let msg = DevMessage::Error {
            errors: vec![DevError {
                file: "app/page.gx".into(),
                line: 10,
                col: 5,
                message: "unexpected token".into(),
                code: Some("GX0002".into()),
                source_line: Some("  let x = ".into()),
                suggestion: None,
            }],
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("app/page.gx"));
        assert!(json.contains("unexpected token"));
    }

    #[test]
    fn backend_down_html_has_overlay() {
        let html = backend_down_html();
        assert!(html.contains("overlay.js"));
        assert!(html.contains("Building"));
    }
}
