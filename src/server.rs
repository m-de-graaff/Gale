use std::net::SocketAddr;
use std::time::Duration;

use axum::middleware;
use axum::Router;
use tokio::net::TcpListener;

use crate::config::Config;
use crate::security::headers::{security_headers_middleware, SecurityHeadersState};
use crate::security::limits::{request_limits_middleware, RequestLimitsState};
use crate::security::path::{path_security_middleware, PathSecurityState};

fn build_cors_layer(config: &crate::config::CorsConfig) -> Option<tower_http::cors::CorsLayer> {
    if !config.enabled {
        return None;
    }

    let mut layer = tower_http::cors::CorsLayer::new();

    // Origins: wildcard prevents browsers from sending credentials on cross-origin
    // requests per CORS spec, so Allow-Origin: * is safe against credential leaks.
    if config.allowed_origins.iter().any(|o| o == "*") {
        layer = layer.allow_origin(tower_http::cors::Any);
    } else if !config.allowed_origins.is_empty() {
        let origins: Vec<axum::http::HeaderValue> = config
            .allowed_origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        layer = layer.allow_origin(origins);
    }
    // Empty origins → no Access-Control-Allow-Origin → browser rejects cross-origin

    let methods: Vec<axum::http::Method> = config
        .allowed_methods
        .iter()
        .filter_map(|m| m.parse().ok())
        .collect();
    layer = layer.allow_methods(methods);

    if !config.allowed_headers.is_empty() {
        let headers: Vec<axum::http::HeaderName> = config
            .allowed_headers
            .iter()
            .filter_map(|h| h.parse().ok())
            .collect();
        layer = layer.allow_headers(headers);
    }

    layer = layer.max_age(std::time::Duration::from_secs(config.max_age));
    Some(layer)
}

pub async fn run(config: Config) {
    let canonical_root = std::path::PathBuf::from(&config.root)
        .canonicalize()
        .unwrap_or_else(|e| {
            eprintln!("Fatal: cannot resolve document root '{}': {e}", config.root);
            std::process::exit(1);
        });

    let security_state = PathSecurityState {
        canonical_root,
        block_dotfiles: config.block_dotfiles,
        canonical_cache: std::sync::Arc::new(dashmap::DashMap::new()),
    };

    let headers_state = SecurityHeadersState::from_config(&config.security_headers);
    let limits_state = RequestLimitsState::from_config(&config.limits);

    #[cfg(feature = "compression")]
    let compression_layer = crate::compression::build_layer(&config.compression);

    let compression_enabled = {
        #[cfg(feature = "compression")]
        { config.compression.enabled }
        #[cfg(not(feature = "compression"))]
        { false }
    };
    let cache_state = crate::cache::CacheState::new(&config.cache, compression_enabled);

    let rate_limit_state = crate::rate_limit::RateLimitState::new(&config.rate_limit);
    crate::rate_limit::spawn_cleanup_task(&rate_limit_state);

    // Inner router: static files + all heavy middleware
    // Middleware stack (outermost listed last):
    // rate_limit → security_headers → CORS → request_limits →
    // path_security → compression → cache → static_files
    let static_app = crate::static_files::create_static_router(&config)
        .layer(middleware::from_fn_with_state(
            cache_state,
            crate::cache::cache_middleware,
        ));
    #[cfg(feature = "compression")]
    let static_app = static_app.layer(compression_layer);
    let static_app = static_app
        .layer(middleware::from_fn_with_state(
            security_state,
            path_security_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            limits_state,
            request_limits_middleware,
        ));

    // Conditionally insert CORS layer between request_limits and security_headers
    let static_app = if let Some(cors_layer) = build_cors_layer(&config.cors) {
        static_app.layer(cors_layer)
    } else {
        static_app
    };

    let static_app = static_app
        .layer(middleware::from_fn_with_state(
            headers_state,
            security_headers_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            rate_limit_state,
            crate::rate_limit::rate_limit_middleware,
        ));

    // Outer router: health route (no heavy middleware) + merge inner
    let mut app = Router::new();
    if !config.health_endpoint.is_empty() {
        app = app.route(
            &config.health_endpoint,
            axum::routing::get(crate::static_files::health_handler),
        );
    }
    let app = app.merge(static_app).layer(middleware::from_fn(
        crate::logging::request_logging_middleware,
    ));

    let addr: SocketAddr = format!("{}:{}", config.bind, config.port)
        .parse()
        .expect("invalid bind address or port");

    let shutdown_timeout = Duration::from_secs(config.shutdown_timeout_secs);

    #[cfg(feature = "tls")]
    if config.tls.enabled {
        run_tls(app, addr, &config).await;
        return;
    }
    run_plain(app, addr, shutdown_timeout).await;
}

/// Run the server with additional application routes.
///
/// The `extra_routes` router is merged **before** the static file
/// fallback, giving explicit application endpoints (actions, API
/// routes, WebSocket upgrades) priority over static file serving.
///
/// The full Gale middleware stack (security headers, path security,
/// request limits, compression, caching, rate limiting, logging,
/// TLS) is applied on top.
///
/// This is the entry point used by GaleX-generated server projects.
pub async fn run_with_app(config: Config, extra_routes: Router) {
    let canonical_root = std::path::PathBuf::from(&config.root)
        .canonicalize()
        .unwrap_or_else(|e| {
            eprintln!("Fatal: cannot resolve document root '{}': {e}", config.root);
            std::process::exit(1);
        });

    let security_state = PathSecurityState {
        canonical_root,
        block_dotfiles: config.block_dotfiles,
        canonical_cache: std::sync::Arc::new(dashmap::DashMap::new()),
    };

    let headers_state = SecurityHeadersState::from_config(&config.security_headers);
    let limits_state = RequestLimitsState::from_config(&config.limits);

    #[cfg(feature = "compression")]
    let compression_layer = crate::compression::build_layer(&config.compression);

    let compression_enabled = {
        #[cfg(feature = "compression")]
        { config.compression.enabled }
        #[cfg(not(feature = "compression"))]
        { false }
    };
    let cache_state = crate::cache::CacheState::new(&config.cache, compression_enabled);

    let rate_limit_state = crate::rate_limit::RateLimitState::new(&config.rate_limit);
    crate::rate_limit::spawn_cleanup_task(&rate_limit_state);

    // Merge extra routes with static file fallback — explicit routes take priority
    let inner_app = extra_routes
        .merge(crate::static_files::create_static_router(&config))
        .layer(middleware::from_fn_with_state(
            cache_state,
            crate::cache::cache_middleware,
        ));
    #[cfg(feature = "compression")]
    let inner_app = inner_app.layer(compression_layer);
    let inner_app = inner_app
        .layer(middleware::from_fn_with_state(
            security_state,
            path_security_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            limits_state,
            request_limits_middleware,
        ));

    let inner_app = if let Some(cors_layer) = build_cors_layer(&config.cors) {
        inner_app.layer(cors_layer)
    } else {
        inner_app
    };

    let inner_app = inner_app
        .layer(middleware::from_fn_with_state(
            headers_state,
            security_headers_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            rate_limit_state,
            crate::rate_limit::rate_limit_middleware,
        ));

    let mut app = Router::new();
    if !config.health_endpoint.is_empty() {
        app = app.route(
            &config.health_endpoint,
            axum::routing::get(crate::static_files::health_handler),
        );
    }
    let app = app.merge(inner_app).layer(middleware::from_fn(
        crate::logging::request_logging_middleware,
    ));

    let addr: SocketAddr = format!("{}:{}", config.bind, config.port)
        .parse()
        .expect("invalid bind address or port");

    let shutdown_timeout = Duration::from_secs(config.shutdown_timeout_secs);

    #[cfg(feature = "tls")]
    if config.tls.enabled {
        run_tls(app, addr, &config).await;
        return;
    }
    run_plain(app, addr, shutdown_timeout).await;
}

async fn run_plain(app: Router, addr: SocketAddr, shutdown_timeout: Duration) {
    let listener = TcpListener::bind(addr)
        .await
        .expect("failed to bind to address");

    tracing::debug!(%addr, "listening");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        crate::platform::shutdown_signal().await;
        let timeout = shutdown_timeout;
        tokio::spawn(async move {
            tokio::time::sleep(timeout).await;
            tracing::warn!(
                timeout_secs = timeout.as_secs(),
                "drain timeout expired, forcing shutdown"
            );
            std::process::exit(0);
        });
    })
    .await
    .expect("server error");
}

#[cfg(feature = "tls")]
async fn run_tls(app: Router, addr: SocketAddr, config: &Config) {
    crate::tls::validate_tls_config(&config.tls);

    let (rustls_config, background_tasks) = if config.tls.acme {
        let (cfg, renewal_task) = crate::tls::build_acme_rustls_config(&config.tls).await;
        (cfg, vec![renewal_task])
    } else {
        let cfg = crate::tls::build_rustls_config(&config.tls).await;
        let reload_task = crate::tls::spawn_cert_reload_task(&config.tls, cfg.clone());
        (cfg, vec![reload_task])
    };

    let handle = axum_server::Handle::new();

    // Spawn shutdown signal task
    let shutdown_handle = handle.clone();
    let shutdown_timeout_secs = config.shutdown_timeout_secs;
    tokio::spawn(async move {
        crate::platform::shutdown_signal().await;
        shutdown_handle.graceful_shutdown(Some(Duration::from_secs(shutdown_timeout_secs)));
    });

    // Spawn HTTP→HTTPS redirect server if redirect_port > 0
    let redirect_task = if config.tls.redirect_port > 0 {
        let redirect_addr: SocketAddr = format!("{}:{}", config.bind, config.tls.redirect_port)
            .parse()
            .expect("invalid bind address or redirect port");

        let redirect_app = crate::tls::redirect_router(config.port);

        Some(tokio::spawn(async move {
            match TcpListener::bind(redirect_addr).await {
                Ok(listener) => {
                    tracing::debug!(%redirect_addr, "HTTP redirect listening");
                    let _ = axum::serve(listener, redirect_app)
                        .with_graceful_shutdown(crate::platform::shutdown_signal())
                        .await;
                }
                Err(e) => {
                    tracing::warn!(
                        %redirect_addr,
                        %e,
                        "could not bind HTTP redirect, continuing with HTTPS only"
                    );
                }
            }
        }))
    } else {
        None
    };

    tracing::debug!(%addr, "HTTPS listening");

    axum_server::bind_rustls(addr, rustls_config)
        .handle(handle)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .expect("TLS server error");

    // Abort background tasks on shutdown
    for task in background_tasks {
        task.abort();
    }

    // Wait for redirect task to clean up
    if let Some(task) = redirect_task {
        let _ = task.await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CorsConfig;

    #[test]
    fn build_cors_layer_disabled_returns_none() {
        let config = CorsConfig {
            enabled: false,
            allowed_origins: vec!["https://example.com".into()],
            allowed_methods: vec!["GET".into()],
            allowed_headers: Vec::new(),
            max_age: 3600,
        };
        assert!(build_cors_layer(&config).is_none());
    }

    #[test]
    fn build_cors_layer_enabled_returns_some() {
        let config = CorsConfig {
            enabled: true,
            allowed_origins: vec!["https://example.com".into()],
            allowed_methods: vec!["GET".into(), "HEAD".into()],
            allowed_headers: Vec::new(),
            max_age: 86400,
        };
        assert!(build_cors_layer(&config).is_some());
    }

    #[test]
    fn build_cors_layer_wildcard_origin_does_not_panic() {
        let config = CorsConfig {
            enabled: true,
            allowed_origins: vec!["*".into()],
            allowed_methods: vec!["GET".into()],
            allowed_headers: vec!["Content-Type".into()],
            max_age: 3600,
        };
        let layer = build_cors_layer(&config);
        assert!(layer.is_some());
    }
}
