use serde::Deserialize;
use std::env;
use std::fs;

#[derive(Debug, Clone)]
pub struct SecurityHeadersConfig {
    pub csp: String,
    pub hsts_max_age: u64,
    pub hsts_include_subdomains: bool,
    pub x_content_type_options: bool,
    pub x_frame_options: String,
    pub referrer_policy: String,
    pub permissions_policy: String,
    pub server_header: String,
}

#[derive(Debug, Clone)]
pub struct LimitsConfig {
    pub max_body_size: u64,
    pub max_uri_length: usize,
    pub max_header_count: usize,
    pub max_header_size: usize,
    pub request_timeout_secs: u64,
    pub read_timeout_secs: u64,
    pub write_timeout_secs: u64,
}

#[derive(Debug, Clone)]
pub struct CompressionConfig {
    pub enabled: bool,
    pub min_size: u64,
    pub algorithms: Vec<String>,
    pub pre_compressed: bool,
    pub skip_extensions: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub default_max_age: u64,
    pub immutable_max_age: u64,
    pub immutable_extensions: Vec<String>,
    pub no_cache_extensions: Vec<String>,
}

#[derive(Debug)]
pub struct Config {
    pub bind: String,
    pub port: u16,
    pub root: String,
    pub index: String,
    pub error_page_404: String,
    pub block_dotfiles: bool,
    pub security_headers: SecurityHeadersConfig,
    pub limits: LimitsConfig,
    pub compression: CompressionConfig,
    pub cache: CacheConfig,
}

#[derive(Deserialize)]
struct FileCompressionConfig {
    enabled: Option<bool>,
    min_size: Option<u64>,
    algorithms: Option<Vec<String>>,
    pre_compressed: Option<bool>,
    skip_extensions: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct FileCacheConfig {
    default_max_age: Option<u64>,
    immutable_max_age: Option<u64>,
    immutable_extensions: Option<Vec<String>>,
    no_cache_extensions: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct FileConfig {
    server: Option<ServerConfig>,
    security: Option<FileSecurityConfig>,
    limits: Option<FileLimitsConfig>,
    compression: Option<FileCompressionConfig>,
    cache: Option<FileCacheConfig>,
}

#[derive(Deserialize)]
struct FileLimitsConfig {
    max_body_size: Option<u64>,
    max_uri_length: Option<usize>,
    max_header_count: Option<usize>,
    max_header_size: Option<usize>,
    request_timeout_secs: Option<u64>,
    read_timeout_secs: Option<u64>,
    write_timeout_secs: Option<u64>,
}

#[derive(Deserialize)]
struct FileSecurityConfig {
    block_dotfiles: Option<bool>,
    csp: Option<String>,
    hsts_max_age: Option<u64>,
    hsts_include_subdomains: Option<bool>,
    x_content_type_options: Option<bool>,
    x_frame_options: Option<String>,
    referrer_policy: Option<String>,
    permissions_policy: Option<String>,
    server_header: Option<String>,
}

#[derive(Deserialize)]
struct ServerConfig {
    bind: Option<String>,
    port: Option<u16>,
    root: Option<String>,
    index: Option<String>,
    error_page_404: Option<String>,
}

impl Config {
    pub fn load() -> Self {
        // Start with defaults
        let mut config = Config {
            bind: "0.0.0.0".to_string(),
            port: 8080,
            root: "./public".to_string(),
            index: "index.html".to_string(),
            error_page_404: String::new(),
            block_dotfiles: true,
            security_headers: SecurityHeadersConfig {
                csp: "default-src 'self'".to_string(),
                hsts_max_age: 31_536_000,
                hsts_include_subdomains: true,
                x_content_type_options: true,
                x_frame_options: "DENY".to_string(),
                referrer_policy: "strict-origin-when-cross-origin".to_string(),
                permissions_policy: "camera=(), microphone=(), geolocation=()".to_string(),
                server_header: String::new(),
            },
            limits: LimitsConfig {
                max_body_size: 10_485_760,
                max_uri_length: 8192,
                max_header_count: 100,
                max_header_size: 8192,
                request_timeout_secs: 30,
                read_timeout_secs: 10,
                write_timeout_secs: 10,
            },
            compression: CompressionConfig {
                enabled: true,
                min_size: 1024,
                algorithms: vec!["br".into(), "gzip".into()],
                pre_compressed: true,
                skip_extensions: vec![
                    "png", "jpg", "jpeg", "gif", "webp", "avif",
                    "woff2", "woff",
                    "mp4", "webm", "ogg",
                    "zip", "gz", "br", "zst",
                ]
                .into_iter()
                .map(String::from)
                .collect(),
            },
            cache: CacheConfig {
                default_max_age: 3600,
                immutable_max_age: 31_536_000,
                immutable_extensions: vec![
                    "js", "css", "woff2", "woff", "ttf", "eot",
                    "png", "jpg", "jpeg", "gif", "svg", "webp", "avif", "ico",
                    "mp4", "webm", "ogg",
                    "wasm",
                ]
                .into_iter()
                .map(String::from)
                .collect(),
                no_cache_extensions: vec!["html", "htm"]
                    .into_iter()
                    .map(String::from)
                    .collect(),
            },
        };

        // Try to load from gale.toml or Gale.toml
        let toml_content = fs::read_to_string("gale.toml")
            .or_else(|_| fs::read_to_string("Gale.toml"))
            .ok();

        if let Some(content) = toml_content {
            match toml::from_str::<FileConfig>(&content) {
                Ok(file_config) => {
                    if let Some(server) = file_config.server {
                        if let Some(bind) = server.bind {
                            config.bind = bind;
                        }
                        if let Some(port) = server.port {
                            config.port = port;
                        }
                        if let Some(root) = server.root {
                            config.root = root;
                        }
                        if let Some(index) = server.index {
                            config.index = index;
                        }
                        if let Some(ep) = server.error_page_404 {
                            config.error_page_404 = ep;
                        }
                    }
                    if let Some(limits) = file_config.limits {
                        if let Some(v) = limits.max_body_size {
                            config.limits.max_body_size = v;
                        }
                        if let Some(v) = limits.max_uri_length {
                            config.limits.max_uri_length = v;
                        }
                        if let Some(v) = limits.max_header_count {
                            config.limits.max_header_count = v;
                        }
                        if let Some(v) = limits.max_header_size {
                            config.limits.max_header_size = v;
                        }
                        if let Some(v) = limits.request_timeout_secs {
                            config.limits.request_timeout_secs = v;
                        }
                        if let Some(v) = limits.read_timeout_secs {
                            config.limits.read_timeout_secs = v;
                        }
                        if let Some(v) = limits.write_timeout_secs {
                            config.limits.write_timeout_secs = v;
                        }
                    }
                    if let Some(security) = file_config.security {
                        if let Some(bd) = security.block_dotfiles {
                            config.block_dotfiles = bd;
                        }
                        if let Some(v) = security.csp {
                            config.security_headers.csp = v;
                        }
                        if let Some(v) = security.hsts_max_age {
                            config.security_headers.hsts_max_age = v;
                        }
                        if let Some(v) = security.hsts_include_subdomains {
                            config.security_headers.hsts_include_subdomains = v;
                        }
                        if let Some(v) = security.x_content_type_options {
                            config.security_headers.x_content_type_options = v;
                        }
                        if let Some(v) = security.x_frame_options {
                            config.security_headers.x_frame_options = v;
                        }
                        if let Some(v) = security.referrer_policy {
                            config.security_headers.referrer_policy = v;
                        }
                        if let Some(v) = security.permissions_policy {
                            config.security_headers.permissions_policy = v;
                        }
                        if let Some(v) = security.server_header {
                            config.security_headers.server_header = v;
                        }
                    }
                    if let Some(compression) = file_config.compression {
                        if let Some(v) = compression.enabled {
                            config.compression.enabled = v;
                        }
                        if let Some(v) = compression.min_size {
                            config.compression.min_size = v;
                        }
                        if let Some(v) = compression.algorithms {
                            config.compression.algorithms = v;
                        }
                        if let Some(v) = compression.pre_compressed {
                            config.compression.pre_compressed = v;
                        }
                        if let Some(v) = compression.skip_extensions {
                            config.compression.skip_extensions = v;
                        }
                    }
                    if let Some(cache) = file_config.cache {
                        if let Some(v) = cache.default_max_age {
                            config.cache.default_max_age = v;
                        }
                        if let Some(v) = cache.immutable_max_age {
                            config.cache.immutable_max_age = v;
                        }
                        if let Some(v) = cache.immutable_extensions {
                            config.cache.immutable_extensions = v;
                        }
                        if let Some(v) = cache.no_cache_extensions {
                            config.cache.no_cache_extensions = v;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Warning: failed to parse config file: {e}");
                }
            }
        }

        // Environment variables override everything
        if let Ok(bind) = env::var("GALE_BIND") {
            config.bind = bind;
        }
        if let Ok(port) = env::var("GALE_PORT") {
            if let Ok(port) = port.parse::<u16>() {
                config.port = port;
            } else {
                eprintln!("Warning: GALE_PORT is not a valid port number, ignoring");
            }
        }
        if let Ok(root) = env::var("GALE_ROOT") {
            config.root = root;
        }
        if let Ok(index) = env::var("GALE_INDEX") {
            config.index = index;
        }
        if let Ok(ep) = env::var("GALE_ERROR_PAGE_404") {
            config.error_page_404 = ep;
        }
        if let Ok(bd) = env::var("GALE_BLOCK_DOTFILES") {
            match bd.as_str() {
                "true" | "1" => config.block_dotfiles = true,
                "false" | "0" => config.block_dotfiles = false,
                _ => eprintln!("Warning: GALE_BLOCK_DOTFILES must be true/false/1/0, ignoring"),
            }
        }
        if let Ok(v) = env::var("GALE_CSP") {
            config.security_headers.csp = v;
        }
        if let Ok(v) = env::var("GALE_HSTS_MAX_AGE") {
            if let Ok(n) = v.parse::<u64>() {
                config.security_headers.hsts_max_age = n;
            } else {
                eprintln!("Warning: GALE_HSTS_MAX_AGE is not a valid number, ignoring");
            }
        }
        if let Ok(v) = env::var("GALE_X_FRAME_OPTIONS") {
            config.security_headers.x_frame_options = v;
        }
        if let Ok(v) = env::var("GALE_SERVER_HEADER") {
            config.security_headers.server_header = v;
        }
        if let Ok(v) = env::var("GALE_MAX_BODY_SIZE") {
            if let Ok(n) = v.parse::<u64>() {
                config.limits.max_body_size = n;
            } else {
                eprintln!("Warning: GALE_MAX_BODY_SIZE is not a valid number, ignoring");
            }
        }
        if let Ok(v) = env::var("GALE_MAX_URI_LENGTH") {
            if let Ok(n) = v.parse::<usize>() {
                config.limits.max_uri_length = n;
            } else {
                eprintln!("Warning: GALE_MAX_URI_LENGTH is not a valid number, ignoring");
            }
        }
        if let Ok(v) = env::var("GALE_MAX_HEADER_COUNT") {
            if let Ok(n) = v.parse::<usize>() {
                config.limits.max_header_count = n;
            } else {
                eprintln!("Warning: GALE_MAX_HEADER_COUNT is not a valid number, ignoring");
            }
        }
        if let Ok(v) = env::var("GALE_MAX_HEADER_SIZE") {
            if let Ok(n) = v.parse::<usize>() {
                config.limits.max_header_size = n;
            } else {
                eprintln!("Warning: GALE_MAX_HEADER_SIZE is not a valid number, ignoring");
            }
        }
        if let Ok(v) = env::var("GALE_REQUEST_TIMEOUT_SECS") {
            if let Ok(n) = v.parse::<u64>() {
                config.limits.request_timeout_secs = n;
            } else {
                eprintln!("Warning: GALE_REQUEST_TIMEOUT_SECS is not a valid number, ignoring");
            }
        }
        if let Ok(v) = env::var("GALE_READ_TIMEOUT_SECS") {
            if let Ok(n) = v.parse::<u64>() {
                config.limits.read_timeout_secs = n;
            } else {
                eprintln!("Warning: GALE_READ_TIMEOUT_SECS is not a valid number, ignoring");
            }
        }
        if let Ok(v) = env::var("GALE_WRITE_TIMEOUT_SECS") {
            if let Ok(n) = v.parse::<u64>() {
                config.limits.write_timeout_secs = n;
            } else {
                eprintln!("Warning: GALE_WRITE_TIMEOUT_SECS is not a valid number, ignoring");
            }
        }
        if let Ok(v) = env::var("GALE_COMPRESSION_ENABLED") {
            match v.as_str() {
                "true" | "1" => config.compression.enabled = true,
                "false" | "0" => config.compression.enabled = false,
                _ => eprintln!(
                    "Warning: GALE_COMPRESSION_ENABLED must be true/false/1/0, ignoring"
                ),
            }
        }
        if let Ok(v) = env::var("GALE_COMPRESSION_MIN_SIZE") {
            if let Ok(n) = v.parse::<u64>() {
                config.compression.min_size = n;
            } else {
                eprintln!("Warning: GALE_COMPRESSION_MIN_SIZE is not a valid number, ignoring");
            }
        }
        if let Ok(v) = env::var("GALE_COMPRESSION_ALGORITHMS") {
            config.compression.algorithms =
                v.split(',').map(|s| s.trim().to_string()).collect();
        }
        if let Ok(v) = env::var("GALE_COMPRESSION_PRE_COMPRESSED") {
            match v.as_str() {
                "true" | "1" => config.compression.pre_compressed = true,
                "false" | "0" => config.compression.pre_compressed = false,
                _ => eprintln!(
                    "Warning: GALE_COMPRESSION_PRE_COMPRESSED must be true/false/1/0, ignoring"
                ),
            }
        }
        if let Ok(v) = env::var("GALE_COMPRESSION_SKIP_EXTENSIONS") {
            config.compression.skip_extensions =
                v.split(',').map(|s| s.trim().to_string()).collect();
        }
        if let Ok(v) = env::var("GALE_CACHE_DEFAULT_MAX_AGE") {
            if let Ok(n) = v.parse::<u64>() {
                config.cache.default_max_age = n;
            } else {
                eprintln!("Warning: GALE_CACHE_DEFAULT_MAX_AGE is not a valid number, ignoring");
            }
        }
        if let Ok(v) = env::var("GALE_CACHE_IMMUTABLE_MAX_AGE") {
            if let Ok(n) = v.parse::<u64>() {
                config.cache.immutable_max_age = n;
            } else {
                eprintln!("Warning: GALE_CACHE_IMMUTABLE_MAX_AGE is not a valid number, ignoring");
            }
        }
        if let Ok(v) = env::var("GALE_CACHE_IMMUTABLE_EXTENSIONS") {
            config.cache.immutable_extensions =
                v.split(',').map(|s| s.trim().to_string()).collect();
        }
        if let Ok(v) = env::var("GALE_CACHE_NO_CACHE_EXTENSIONS") {
            config.cache.no_cache_extensions =
                v.split(',').map(|s| s.trim().to_string()).collect();
        }

        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Config::load reads env vars, so tests that set env vars must not run in parallel.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn clear_compression_env_vars() {
        env::remove_var("GALE_COMPRESSION_ENABLED");
        env::remove_var("GALE_COMPRESSION_MIN_SIZE");
        env::remove_var("GALE_COMPRESSION_ALGORITHMS");
        env::remove_var("GALE_COMPRESSION_PRE_COMPRESSED");
        env::remove_var("GALE_COMPRESSION_SKIP_EXTENSIONS");
    }

    fn clear_cache_env_vars() {
        env::remove_var("GALE_CACHE_DEFAULT_MAX_AGE");
        env::remove_var("GALE_CACHE_IMMUTABLE_MAX_AGE");
        env::remove_var("GALE_CACHE_IMMUTABLE_EXTENSIONS");
        env::remove_var("GALE_CACHE_NO_CACHE_EXTENSIONS");
    }

    #[test]
    fn compression_defaults_match_gale_toml() {
        let _lock = ENV_LOCK.lock().unwrap();
        clear_compression_env_vars();

        let config = Config::load();
        assert!(config.compression.enabled);
        assert_eq!(config.compression.min_size, 1024);
        assert_eq!(config.compression.algorithms, vec!["br", "gzip"]);
        assert!(config.compression.pre_compressed);
        assert!(config.compression.skip_extensions.contains(&"png".to_string()));
        assert!(config.compression.skip_extensions.contains(&"woff2".to_string()));
        assert!(config.compression.skip_extensions.contains(&"mp4".to_string()));
        assert!(config.compression.skip_extensions.contains(&"zip".to_string()));
    }

    #[test]
    fn env_compression_enabled_false_disables() {
        let _lock = ENV_LOCK.lock().unwrap();
        clear_compression_env_vars();
        env::set_var("GALE_COMPRESSION_ENABLED", "false");

        let config = Config::load();
        assert!(!config.compression.enabled);

        env::remove_var("GALE_COMPRESSION_ENABLED");
    }

    #[test]
    fn env_compression_algorithms_comma_parsing() {
        let _lock = ENV_LOCK.lock().unwrap();
        clear_compression_env_vars();
        env::set_var("GALE_COMPRESSION_ALGORITHMS", "gzip, br");

        let config = Config::load();
        assert_eq!(config.compression.algorithms, vec!["gzip", "br"]);

        env::remove_var("GALE_COMPRESSION_ALGORITHMS");
    }

    #[test]
    fn env_compression_min_size_override() {
        let _lock = ENV_LOCK.lock().unwrap();
        clear_compression_env_vars();
        env::set_var("GALE_COMPRESSION_MIN_SIZE", "512");

        let config = Config::load();
        assert_eq!(config.compression.min_size, 512);

        env::remove_var("GALE_COMPRESSION_MIN_SIZE");
    }

    #[test]
    fn cache_defaults_match_gale_toml() {
        let _lock = ENV_LOCK.lock().unwrap();
        clear_cache_env_vars();

        let config = Config::load();
        assert_eq!(config.cache.default_max_age, 3600);
        assert_eq!(config.cache.immutable_max_age, 31_536_000);
        assert!(config.cache.immutable_extensions.contains(&"js".to_string()));
        assert!(config.cache.immutable_extensions.contains(&"css".to_string()));
        assert!(config.cache.immutable_extensions.contains(&"woff2".to_string()));
        assert!(config.cache.immutable_extensions.contains(&"wasm".to_string()));
        assert!(config.cache.no_cache_extensions.contains(&"html".to_string()));
        assert!(config.cache.no_cache_extensions.contains(&"htm".to_string()));
    }

    #[test]
    fn env_cache_default_max_age_override() {
        let _lock = ENV_LOCK.lock().unwrap();
        clear_cache_env_vars();
        env::set_var("GALE_CACHE_DEFAULT_MAX_AGE", "7200");

        let config = Config::load();
        assert_eq!(config.cache.default_max_age, 7200);

        env::remove_var("GALE_CACHE_DEFAULT_MAX_AGE");
    }

    #[test]
    fn env_cache_immutable_extensions_comma_parsing() {
        let _lock = ENV_LOCK.lock().unwrap();
        clear_cache_env_vars();
        env::set_var("GALE_CACHE_IMMUTABLE_EXTENSIONS", "js, css, woff2");

        let config = Config::load();
        assert_eq!(
            config.cache.immutable_extensions,
            vec!["js", "css", "woff2"]
        );

        env::remove_var("GALE_CACHE_IMMUTABLE_EXTENSIONS");
    }
}
