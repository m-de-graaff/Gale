use std::net::SocketAddr;

use axum::middleware;
use tokio::net::TcpListener;

use crate::config::Config;
use crate::security::headers::{SecurityHeadersState, security_headers_middleware};
use crate::security::limits::{RequestLimitsState, request_limits_middleware};
use crate::security::path::{PathSecurityState, path_security_middleware};

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
    };

    let headers_state = SecurityHeadersState::from_config(&config.security_headers);
    let limits_state = RequestLimitsState::from_config(&config.limits);

    let compression_layer = crate::compression::build_layer(&config.compression);

    let cache_state = crate::cache::CacheState::new(&config.cache, config.compression.enabled);

    let app = crate::static_files::create_router(&config)
        .layer(middleware::from_fn_with_state(
            cache_state,
            crate::cache::cache_middleware,
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
        ));

    let addr: SocketAddr = format!("{}:{}", config.bind, config.port)
        .parse()
        .expect("invalid bind address or port");

    let listener = TcpListener::bind(addr)
        .await
        .expect("failed to bind to address");

    println!("Listening on {addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(crate::platform::shutdown_signal())
        .await
        .expect("server error");
}
