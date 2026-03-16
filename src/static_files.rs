use axum::Router;
use tower_http::services::ServeDir;

use crate::config::Config;
use crate::error::NotFoundService;

pub fn create_router(config: &Config) -> Router {
    let not_found = NotFoundService::from_config(&config.error_page_404);

    let mut serve_dir = ServeDir::new(&config.root);

    if config.compression.pre_compressed {
        if config.compression.algorithms.iter().any(|a| a == "br") {
            serve_dir = serve_dir.precompressed_br();
        }
        if config.compression.algorithms.iter().any(|a| a == "gzip") {
            serve_dir = serve_dir.precompressed_gzip();
        }
    }

    let serve_dir = serve_dir
        .append_index_html_on_directories(true)
        .not_found_service(not_found);

    Router::new().fallback_service(serve_dir)
}
