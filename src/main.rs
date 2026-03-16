mod cache;
mod compression;
mod config;
mod error;
mod mime_types;
mod platform;
mod security;
mod server;
mod static_files;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = config::Config::load();
    println!("Gale starting on {}:{}", config.bind, config.port);
    println!("Serving from: {}", config.root);
    server::run(config).await;
}
