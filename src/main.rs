use gale_lib::config;
use gale_lib::logging;
use gale_lib::server;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
async fn main() {
    // Install ring as the default crypto provider for rustls.
    // Must happen before any TLS config creation (axum-server, rustls-acme).
    #[cfg(feature = "tls")]
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("failed to install rustls crypto provider");

    let config = config::Config::load();
    logging::init(&config.logging);

    let protocol = if config.tls.enabled { "https" } else { "http" };
    tracing::info!(
        bind = %config.bind,
        port = config.port,
        root = %config.root,
        "gale starting on {protocol}",
    );
    server::run(config).await;
}
