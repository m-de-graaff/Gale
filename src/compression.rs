use std::collections::HashSet;

use axum::http::{Response, header};
use tower_http::compression::CompressionLayer;
use tower_http::compression::predicate::Predicate;

use crate::config::CompressionConfig;
use crate::mime_types;

/// MIME type prefixes/exact matches that benefit from compression.
const COMPRESSIBLE_TYPES: &[&str] = &[
    "text/",
    "application/javascript",
    "application/json",
    "application/xml",
    "application/xhtml+xml",
    "application/manifest+json",
    "application/wasm",
    "application/rss+xml",
    "application/atom+xml",
    "image/svg+xml",
];

/// Predicate that decides whether a response should be compressed.
#[derive(Clone, Debug)]
pub struct ShouldCompress {
    enabled: bool,
    min_size: u64,
    skip_content_types: HashSet<String>,
}

impl ShouldCompress {
    pub fn from_config(config: &CompressionConfig) -> Self {
        let skip_content_types: HashSet<String> = config
            .skip_extensions
            .iter()
            .map(|ext| {
                let mime = mime_types::from_extension(ext);
                // Strip charset parameter (e.g., "text/html; charset=utf-8" -> "text/html")
                mime.split(';').next().unwrap_or(mime).trim().to_string()
            })
            .collect();

        Self {
            enabled: config.enabled,
            min_size: config.min_size,
            skip_content_types,
        }
    }

    fn should_compress<B>(&self, response: &Response<B>) -> bool {
        if !self.enabled {
            return false;
        }

        // Check Content-Length against min_size
        if let Some(content_length) = response.headers().get(header::CONTENT_LENGTH) {
            if let Ok(len) = content_length.to_str().unwrap_or("0").parse::<u64>() {
                if len < self.min_size {
                    return false;
                }
            }
        }

        // Check Content-Type
        let content_type = match response.headers().get(header::CONTENT_TYPE) {
            Some(ct) => ct.to_str().unwrap_or(""),
            None => return false,
        };

        // Extract media type (before ";")
        let media_type = content_type.split(';').next().unwrap_or("").trim();

        // Check skip list
        if self.skip_content_types.contains(media_type) {
            return false;
        }

        // Check compressible whitelist
        for compressible in COMPRESSIBLE_TYPES {
            if compressible.ends_with('/') {
                // Prefix match (e.g., "text/")
                if media_type.starts_with(compressible) {
                    return true;
                }
            } else {
                // Exact match
                if media_type == *compressible {
                    return true;
                }
            }
        }

        false
    }
}

impl Predicate for ShouldCompress {
    fn should_compress<B>(&self, response: &Response<B>) -> bool {
        self.should_compress(response)
    }
}

/// Build the compression layer from config.
///
/// The layer is always constructed (even when `enabled = false`) to keep
/// a uniform service type. The predicate short-circuits to `false` when
/// disabled, so the overhead is negligible.
pub fn build_layer(config: &CompressionConfig) -> CompressionLayer<ShouldCompress> {
    let predicate = ShouldCompress::from_config(config);

    // Configure algorithm toggles before setting the predicate,
    // since no_br()/no_gzip() are only available on CompressionLayer<DefaultPredicate>.
    let mut layer = CompressionLayer::new();
    if !config.algorithms.iter().any(|a| a == "br") {
        layer = layer.no_br();
    }
    if !config.algorithms.iter().any(|a| a == "gzip") {
        layer = layer.no_gzip();
    }
    layer = layer.no_deflate().no_zstd();

    layer.compress_when(predicate)
}

#[cfg(test)]
mod tests {
    use axum::http::{Response, StatusCode, header};

    use super::*;

    fn default_config() -> CompressionConfig {
        CompressionConfig {
            enabled: true,
            min_size: 1024,
            algorithms: vec!["br".into(), "gzip".into()],
            pre_compressed: true,
            skip_extensions: vec![
                "png", "jpg", "jpeg", "gif", "webp", "avif", "woff2", "woff", "mp4", "webm",
                "ogg", "zip", "gz", "br", "zst",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        }
    }

    fn response_with(content_type: &str, content_length: Option<u64>) -> Response<()> {
        let mut builder = Response::builder().status(StatusCode::OK);
        builder = builder.header(header::CONTENT_TYPE, content_type);
        if let Some(len) = content_length {
            builder = builder.header(header::CONTENT_LENGTH, len.to_string());
        }
        builder.body(()).unwrap()
    }

    #[test]
    fn compress_text_html_above_min_size() {
        let predicate = ShouldCompress::from_config(&default_config());
        let resp = response_with("text/html; charset=utf-8", Some(2048));
        assert!(predicate.should_compress(&resp));
    }

    #[test]
    fn compress_application_json_above_min_size() {
        let predicate = ShouldCompress::from_config(&default_config());
        let resp = response_with("application/json", Some(2048));
        assert!(predicate.should_compress(&resp));
    }

    #[test]
    fn skip_image_png() {
        let predicate = ShouldCompress::from_config(&default_config());
        let resp = response_with("image/png", Some(50000));
        assert!(!predicate.should_compress(&resp));
    }

    #[test]
    fn skip_below_min_size() {
        let predicate = ShouldCompress::from_config(&default_config());
        let resp = response_with("text/html", Some(512));
        assert!(!predicate.should_compress(&resp));
    }

    #[test]
    fn skip_no_content_type() {
        let predicate = ShouldCompress::from_config(&default_config());
        let resp = Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_LENGTH, "2048")
            .body(())
            .unwrap();
        assert!(!predicate.should_compress(&resp));
    }

    #[test]
    fn skip_when_disabled() {
        let mut config = default_config();
        config.enabled = false;
        let predicate = ShouldCompress::from_config(&config);
        let resp = response_with("text/html", Some(2048));
        assert!(!predicate.should_compress(&resp));
    }

    #[test]
    fn handles_charset_parameter() {
        let predicate = ShouldCompress::from_config(&default_config());
        let resp = response_with("text/css; charset=utf-8", Some(2048));
        assert!(predicate.should_compress(&resp));
    }

    #[test]
    fn build_layer_does_not_panic() {
        let _ = build_layer(&default_config());
    }
}
