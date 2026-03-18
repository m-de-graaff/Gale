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

#[derive(Debug, Clone)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
    pub output: String,
    pub file_path: String,
}

#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub enabled: bool,
    pub requests_per_second: u32,
    pub burst: u32,
    pub max_connections_per_ip: u32,
}

#[derive(Debug, Clone)]
pub struct CorsConfig {
    pub enabled: bool,
    pub allowed_origins: Vec<String>,
    pub allowed_methods: Vec<String>,
    pub allowed_headers: Vec<String>,
    pub max_age: u64,
}

#[derive(Debug, Clone)]
pub struct TlsConfig {
    pub enabled: bool,
    pub cert: String,
    pub key: String,
    pub redirect_port: u16,
    pub acme: bool,
    pub acme_email: String,
    pub acme_domain: String,
    pub acme_cache_dir: String,
    pub acme_production: bool,
}

#[derive(Debug)]
pub struct Config {
    pub bind: String,
    pub port: u16,
    pub root: String,
    pub index: String,
    pub error_page_404: String,
    pub health_endpoint: String,
    pub shutdown_timeout_secs: u64,
    pub block_dotfiles: bool,
    pub security_headers: SecurityHeadersConfig,
    pub limits: LimitsConfig,
    pub tls: TlsConfig,
    pub compression: CompressionConfig,
    pub cache: CacheConfig,
    pub logging: LoggingConfig,
    pub rate_limit: RateLimitConfig,
    pub cors: CorsConfig,
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
struct FileLoggingConfig {
    level: Option<String>,
    format: Option<String>,
    output: Option<String>,
    file_path: Option<String>,
}

#[derive(Deserialize)]
struct FileRateLimitConfig {
    enabled: Option<bool>,
    requests_per_second: Option<u32>,
    burst: Option<u32>,
    max_connections_per_ip: Option<u32>,
}

#[derive(Deserialize)]
struct FileCorsConfig {
    enabled: Option<bool>,
    allowed_origins: Option<Vec<String>>,
    allowed_methods: Option<Vec<String>>,
    allowed_headers: Option<Vec<String>>,
    max_age: Option<u64>,
}

#[derive(Deserialize)]
struct FileTlsConfig {
    enabled: Option<bool>,
    cert: Option<String>,
    key: Option<String>,
    redirect_port: Option<u16>,
    acme: Option<bool>,
    acme_email: Option<String>,
    acme_domain: Option<String>,
    acme_cache_dir: Option<String>,
    acme_production: Option<bool>,
}

#[derive(Deserialize)]
struct FileConfig {
    server: Option<ServerConfig>,
    tls: Option<FileTlsConfig>,
    security: Option<FileSecurityConfig>,
    limits: Option<FileLimitsConfig>,
    compression: Option<FileCompressionConfig>,
    cache: Option<FileCacheConfig>,
    logging: Option<FileLoggingConfig>,
    rate_limit: Option<FileRateLimitConfig>,
    cors: Option<FileCorsConfig>,
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
    health_endpoint: Option<String>,
    shutdown_timeout_secs: Option<u64>,
}

impl Config {
    pub fn defaults() -> Self {
        Config {
            bind: "0.0.0.0".to_string(),
            port: 8080,
            root: "./public".to_string(),
            index: "index.html".to_string(),
            error_page_404: String::new(),
            health_endpoint: "/health".to_string(),
            shutdown_timeout_secs: 10,
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
            tls: TlsConfig {
                enabled: false,
                cert: String::new(),
                key: String::new(),
                redirect_port: 80,
                acme: false,
                acme_email: String::new(),
                acme_domain: String::new(),
                acme_cache_dir: "./acme_cache".to_string(),
                acme_production: false,
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
                    "png", "jpg", "jpeg", "gif", "webp", "avif", "woff2", "woff", "mp4", "webm",
                    "ogg", "zip", "gz", "br", "zst",
                ]
                .into_iter()
                .map(String::from)
                .collect(),
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "clf".to_string(),
                output: "stdout".to_string(),
                file_path: String::new(),
            },
            rate_limit: RateLimitConfig {
                enabled: true,
                requests_per_second: 100,
                burst: 50,
                max_connections_per_ip: 256,
            },
            cors: CorsConfig {
                enabled: false,
                allowed_origins: Vec::new(),
                allowed_methods: vec!["GET".into(), "HEAD".into(), "OPTIONS".into()],
                allowed_headers: Vec::new(),
                max_age: 86400,
            },
            cache: CacheConfig {
                default_max_age: 3600,
                immutable_max_age: 31_536_000,
                immutable_extensions: vec![
                    "js", "css", "woff2", "woff", "ttf", "eot", "png", "jpg", "jpeg", "gif", "svg",
                    "webp", "avif", "ico", "mp4", "webm", "ogg", "wasm",
                ]
                .into_iter()
                .map(String::from)
                .collect(),
                no_cache_extensions: vec!["html", "htm"].into_iter().map(String::from).collect(),
            },
        }
    }

    pub fn load() -> Self {
        let mut config = Self::defaults();

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
                        if let Some(v) = server.health_endpoint {
                            config.health_endpoint = v;
                        }
                        if let Some(v) = server.shutdown_timeout_secs {
                            config.shutdown_timeout_secs = v;
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
                    if let Some(tls) = file_config.tls {
                        if let Some(v) = tls.enabled {
                            config.tls.enabled = v;
                        }
                        if let Some(v) = tls.cert {
                            config.tls.cert = v;
                        }
                        if let Some(v) = tls.key {
                            config.tls.key = v;
                        }
                        if let Some(v) = tls.redirect_port {
                            config.tls.redirect_port = v;
                        }
                        if let Some(v) = tls.acme {
                            config.tls.acme = v;
                        }
                        if let Some(v) = tls.acme_email {
                            config.tls.acme_email = v;
                        }
                        if let Some(v) = tls.acme_domain {
                            config.tls.acme_domain = v;
                        }
                        if let Some(v) = tls.acme_cache_dir {
                            config.tls.acme_cache_dir = v;
                        }
                        if let Some(v) = tls.acme_production {
                            config.tls.acme_production = v;
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
                    if let Some(logging) = file_config.logging {
                        if let Some(v) = logging.level {
                            config.logging.level = v;
                        }
                        if let Some(v) = logging.format {
                            config.logging.format = v;
                        }
                        if let Some(v) = logging.output {
                            config.logging.output = v;
                        }
                        if let Some(v) = logging.file_path {
                            config.logging.file_path = v;
                        }
                    }
                    if let Some(rate_limit) = file_config.rate_limit {
                        if let Some(v) = rate_limit.enabled {
                            config.rate_limit.enabled = v;
                        }
                        if let Some(v) = rate_limit.requests_per_second {
                            config.rate_limit.requests_per_second = v;
                        }
                        if let Some(v) = rate_limit.burst {
                            config.rate_limit.burst = v;
                        }
                        if let Some(v) = rate_limit.max_connections_per_ip {
                            config.rate_limit.max_connections_per_ip = v;
                        }
                    }
                    if let Some(cors) = file_config.cors {
                        if let Some(v) = cors.enabled {
                            config.cors.enabled = v;
                        }
                        if let Some(v) = cors.allowed_origins {
                            config.cors.allowed_origins = v;
                        }
                        if let Some(v) = cors.allowed_methods {
                            config.cors.allowed_methods = v;
                        }
                        if let Some(v) = cors.allowed_headers {
                            config.cors.allowed_headers = v;
                        }
                        if let Some(v) = cors.max_age {
                            config.cors.max_age = v;
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
        if let Ok(v) = env::var("GALE_HEALTH_ENDPOINT") {
            config.health_endpoint = v;
        }
        if let Ok(v) = env::var("GALE_SHUTDOWN_TIMEOUT_SECS") {
            if let Ok(n) = v.parse::<u64>() {
                config.shutdown_timeout_secs = n;
            } else {
                eprintln!("Warning: GALE_SHUTDOWN_TIMEOUT_SECS is not a valid number, ignoring");
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
        if let Ok(v) = env::var("GALE_TLS_ENABLED") {
            match v.as_str() {
                "true" | "1" => config.tls.enabled = true,
                "false" | "0" => config.tls.enabled = false,
                _ => eprintln!("Warning: GALE_TLS_ENABLED must be true/false/1/0, ignoring"),
            }
        }
        if let Ok(v) = env::var("GALE_TLS_CERT") {
            config.tls.cert = v;
        }
        if let Ok(v) = env::var("GALE_TLS_KEY") {
            config.tls.key = v;
        }
        if let Ok(v) = env::var("GALE_TLS_REDIRECT_PORT") {
            if let Ok(n) = v.parse::<u16>() {
                config.tls.redirect_port = n;
            } else {
                eprintln!("Warning: GALE_TLS_REDIRECT_PORT is not a valid port number, ignoring");
            }
        }
        if let Ok(v) = env::var("GALE_TLS_ACME") {
            match v.as_str() {
                "true" | "1" => config.tls.acme = true,
                "false" | "0" => config.tls.acme = false,
                _ => eprintln!("Warning: GALE_TLS_ACME must be true/false/1/0, ignoring"),
            }
        }
        if let Ok(v) = env::var("GALE_TLS_ACME_EMAIL") {
            config.tls.acme_email = v;
        }
        if let Ok(v) = env::var("GALE_TLS_ACME_DOMAIN") {
            config.tls.acme_domain = v;
        }
        if let Ok(v) = env::var("GALE_TLS_ACME_CACHE_DIR") {
            config.tls.acme_cache_dir = v;
        }
        if let Ok(v) = env::var("GALE_TLS_ACME_PRODUCTION") {
            match v.as_str() {
                "true" | "1" => config.tls.acme_production = true,
                "false" | "0" => config.tls.acme_production = false,
                _ => {
                    eprintln!("Warning: GALE_TLS_ACME_PRODUCTION must be true/false/1/0, ignoring")
                }
            }
        }
        if let Ok(v) = env::var("GALE_COMPRESSION_ENABLED") {
            match v.as_str() {
                "true" | "1" => config.compression.enabled = true,
                "false" | "0" => config.compression.enabled = false,
                _ => {
                    eprintln!("Warning: GALE_COMPRESSION_ENABLED must be true/false/1/0, ignoring")
                }
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
            config.compression.algorithms = v.split(',').map(|s| s.trim().to_string()).collect();
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
            config.cache.no_cache_extensions = v.split(',').map(|s| s.trim().to_string()).collect();
        }
        if let Ok(v) = env::var("GALE_LOG_LEVEL") {
            config.logging.level = v;
        }
        if let Ok(v) = env::var("GALE_LOG_FORMAT") {
            config.logging.format = v;
        }
        if let Ok(v) = env::var("GALE_LOG_OUTPUT") {
            config.logging.output = v;
        }
        if let Ok(v) = env::var("GALE_LOG_FILE_PATH") {
            config.logging.file_path = v;
        }
        if let Ok(v) = env::var("GALE_RATE_LIMIT_ENABLED") {
            match v.as_str() {
                "true" | "1" => config.rate_limit.enabled = true,
                "false" | "0" => config.rate_limit.enabled = false,
                _ => eprintln!("Warning: GALE_RATE_LIMIT_ENABLED must be true/false/1/0, ignoring"),
            }
        }
        if let Ok(v) = env::var("GALE_RATE_LIMIT_REQUESTS_PER_SECOND") {
            if let Ok(n) = v.parse::<u32>() {
                config.rate_limit.requests_per_second = n;
            } else {
                eprintln!(
                    "Warning: GALE_RATE_LIMIT_REQUESTS_PER_SECOND is not a valid number, ignoring"
                );
            }
        }
        if let Ok(v) = env::var("GALE_RATE_LIMIT_BURST") {
            if let Ok(n) = v.parse::<u32>() {
                config.rate_limit.burst = n;
            } else {
                eprintln!("Warning: GALE_RATE_LIMIT_BURST is not a valid number, ignoring");
            }
        }
        if let Ok(v) = env::var("GALE_RATE_LIMIT_MAX_CONNECTIONS_PER_IP") {
            if let Ok(n) = v.parse::<u32>() {
                config.rate_limit.max_connections_per_ip = n;
            } else {
                eprintln!(
                    "Warning: GALE_RATE_LIMIT_MAX_CONNECTIONS_PER_IP is not a valid number, ignoring"
                );
            }
        }

        if let Ok(v) = env::var("GALE_CORS_ENABLED") {
            match v.as_str() {
                "true" | "1" => config.cors.enabled = true,
                "false" | "0" => config.cors.enabled = false,
                _ => eprintln!("Warning: GALE_CORS_ENABLED must be true/false/1/0, ignoring"),
            }
        }
        if let Ok(v) = env::var("GALE_CORS_ALLOWED_ORIGINS") {
            config.cors.allowed_origins = v.split(',').map(|s| s.trim().to_string()).collect();
        }
        if let Ok(v) = env::var("GALE_CORS_ALLOWED_METHODS") {
            config.cors.allowed_methods = v.split(',').map(|s| s.trim().to_string()).collect();
        }
        if let Ok(v) = env::var("GALE_CORS_ALLOWED_HEADERS") {
            config.cors.allowed_headers = v.split(',').map(|s| s.trim().to_string()).collect();
        }
        if let Ok(v) = env::var("GALE_CORS_MAX_AGE") {
            if let Ok(n) = v.parse::<u64>() {
                config.cors.max_age = n;
            } else {
                eprintln!("Warning: GALE_CORS_MAX_AGE is not a valid number, ignoring");
            }
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

    fn clear_server_env_vars() {
        env::remove_var("GALE_HEALTH_ENDPOINT");
        env::remove_var("GALE_SHUTDOWN_TIMEOUT_SECS");
    }

    fn clear_tls_env_vars() {
        env::remove_var("GALE_TLS_ENABLED");
        env::remove_var("GALE_TLS_CERT");
        env::remove_var("GALE_TLS_KEY");
        env::remove_var("GALE_TLS_REDIRECT_PORT");
        env::remove_var("GALE_TLS_ACME");
        env::remove_var("GALE_TLS_ACME_EMAIL");
        env::remove_var("GALE_TLS_ACME_DOMAIN");
        env::remove_var("GALE_TLS_ACME_CACHE_DIR");
        env::remove_var("GALE_TLS_ACME_PRODUCTION");
    }

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

    fn clear_logging_env_vars() {
        env::remove_var("GALE_LOG_LEVEL");
        env::remove_var("GALE_LOG_FORMAT");
        env::remove_var("GALE_LOG_OUTPUT");
        env::remove_var("GALE_LOG_FILE_PATH");
    }

    fn clear_rate_limit_env_vars() {
        env::remove_var("GALE_RATE_LIMIT_ENABLED");
        env::remove_var("GALE_RATE_LIMIT_REQUESTS_PER_SECOND");
        env::remove_var("GALE_RATE_LIMIT_BURST");
        env::remove_var("GALE_RATE_LIMIT_MAX_CONNECTIONS_PER_IP");
    }

    #[test]
    fn tls_defaults() {
        let _lock = ENV_LOCK.lock().unwrap();
        clear_tls_env_vars();

        let config = Config::load();
        assert!(!config.tls.enabled);
        assert!(config.tls.cert.is_empty());
        assert!(config.tls.key.is_empty());
        assert_eq!(config.tls.redirect_port, 80);
        assert!(!config.tls.acme);
        assert!(config.tls.acme_email.is_empty());
        assert!(config.tls.acme_domain.is_empty());
        assert_eq!(config.tls.acme_cache_dir, "./acme_cache");
        assert!(!config.tls.acme_production);
    }

    #[test]
    fn env_tls_enabled_override() {
        let _lock = ENV_LOCK.lock().unwrap();
        clear_tls_env_vars();
        env::set_var("GALE_TLS_ENABLED", "true");

        let config = Config::load();
        assert!(config.tls.enabled);

        env::remove_var("GALE_TLS_ENABLED");
    }

    #[test]
    fn env_tls_cert_and_key_override() {
        let _lock = ENV_LOCK.lock().unwrap();
        clear_tls_env_vars();
        env::set_var("GALE_TLS_CERT", "/path/to/cert.pem");
        env::set_var("GALE_TLS_KEY", "/path/to/key.pem");

        let config = Config::load();
        assert_eq!(config.tls.cert, "/path/to/cert.pem");
        assert_eq!(config.tls.key, "/path/to/key.pem");

        env::remove_var("GALE_TLS_CERT");
        env::remove_var("GALE_TLS_KEY");
    }

    #[test]
    fn env_tls_redirect_port_override() {
        let _lock = ENV_LOCK.lock().unwrap();
        clear_tls_env_vars();
        env::set_var("GALE_TLS_REDIRECT_PORT", "8080");

        let config = Config::load();
        assert_eq!(config.tls.redirect_port, 8080);

        env::remove_var("GALE_TLS_REDIRECT_PORT");
    }

    #[test]
    fn env_acme_overrides() {
        let _lock = ENV_LOCK.lock().unwrap();
        clear_tls_env_vars();
        env::set_var("GALE_TLS_ACME", "true");
        env::set_var("GALE_TLS_ACME_EMAIL", "test@example.com");
        env::set_var("GALE_TLS_ACME_DOMAIN", "example.com");
        env::set_var("GALE_TLS_ACME_CACHE_DIR", "/tmp/acme");
        env::set_var("GALE_TLS_ACME_PRODUCTION", "true");

        let config = Config::load();
        assert!(config.tls.acme);
        assert_eq!(config.tls.acme_email, "test@example.com");
        assert_eq!(config.tls.acme_domain, "example.com");
        assert_eq!(config.tls.acme_cache_dir, "/tmp/acme");
        assert!(config.tls.acme_production);

        env::remove_var("GALE_TLS_ACME");
        env::remove_var("GALE_TLS_ACME_EMAIL");
        env::remove_var("GALE_TLS_ACME_DOMAIN");
        env::remove_var("GALE_TLS_ACME_CACHE_DIR");
        env::remove_var("GALE_TLS_ACME_PRODUCTION");
    }

    #[test]
    fn env_acme_production_bool_parsing() {
        let _lock = ENV_LOCK.lock().unwrap();
        clear_tls_env_vars();
        env::set_var("GALE_TLS_ACME_PRODUCTION", "1");

        let config = Config::load();
        assert!(config.tls.acme_production);

        env::set_var("GALE_TLS_ACME_PRODUCTION", "0");
        let config = Config::load();
        assert!(!config.tls.acme_production);

        env::remove_var("GALE_TLS_ACME_PRODUCTION");
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
        assert!(config
            .compression
            .skip_extensions
            .contains(&"png".to_string()));
        assert!(config
            .compression
            .skip_extensions
            .contains(&"woff2".to_string()));
        assert!(config
            .compression
            .skip_extensions
            .contains(&"mp4".to_string()));
        assert!(config
            .compression
            .skip_extensions
            .contains(&"zip".to_string()));
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
        assert!(config
            .cache
            .immutable_extensions
            .contains(&"js".to_string()));
        assert!(config
            .cache
            .immutable_extensions
            .contains(&"css".to_string()));
        assert!(config
            .cache
            .immutable_extensions
            .contains(&"woff2".to_string()));
        assert!(config
            .cache
            .immutable_extensions
            .contains(&"wasm".to_string()));
        assert!(config
            .cache
            .no_cache_extensions
            .contains(&"html".to_string()));
        assert!(config
            .cache
            .no_cache_extensions
            .contains(&"htm".to_string()));
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
    fn logging_defaults() {
        let _lock = ENV_LOCK.lock().unwrap();
        clear_logging_env_vars();

        let config = Config::load();
        assert_eq!(config.logging.level, "info");
        assert_eq!(config.logging.format, "clf");
        assert_eq!(config.logging.output, "stdout");
        assert!(config.logging.file_path.is_empty());
    }

    #[test]
    fn env_log_level_override() {
        let _lock = ENV_LOCK.lock().unwrap();
        clear_logging_env_vars();
        env::set_var("GALE_LOG_LEVEL", "debug");

        let config = Config::load();
        assert_eq!(config.logging.level, "debug");

        env::remove_var("GALE_LOG_LEVEL");
    }

    #[test]
    fn env_log_format_override() {
        let _lock = ENV_LOCK.lock().unwrap();
        clear_logging_env_vars();
        env::set_var("GALE_LOG_FORMAT", "json");

        let config = Config::load();
        assert_eq!(config.logging.format, "json");

        env::remove_var("GALE_LOG_FORMAT");
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

    #[test]
    fn rate_limit_defaults() {
        let _lock = ENV_LOCK.lock().unwrap();
        clear_rate_limit_env_vars();

        let config = Config::load();
        assert!(config.rate_limit.enabled);
        assert_eq!(config.rate_limit.requests_per_second, 100);
        assert_eq!(config.rate_limit.burst, 50);
        assert_eq!(config.rate_limit.max_connections_per_ip, 256);
    }

    #[test]
    fn env_rate_limit_enabled_override() {
        let _lock = ENV_LOCK.lock().unwrap();
        clear_rate_limit_env_vars();
        env::set_var("GALE_RATE_LIMIT_ENABLED", "false");

        let config = Config::load();
        assert!(!config.rate_limit.enabled);

        env::remove_var("GALE_RATE_LIMIT_ENABLED");
    }

    #[test]
    fn env_rate_limit_rps_override() {
        let _lock = ENV_LOCK.lock().unwrap();
        clear_rate_limit_env_vars();
        env::set_var("GALE_RATE_LIMIT_REQUESTS_PER_SECOND", "200");

        let config = Config::load();
        assert_eq!(config.rate_limit.requests_per_second, 200);

        env::remove_var("GALE_RATE_LIMIT_REQUESTS_PER_SECOND");
    }

    #[test]
    fn env_rate_limit_burst_override() {
        let _lock = ENV_LOCK.lock().unwrap();
        clear_rate_limit_env_vars();
        env::set_var("GALE_RATE_LIMIT_BURST", "100");

        let config = Config::load();
        assert_eq!(config.rate_limit.burst, 100);

        env::remove_var("GALE_RATE_LIMIT_BURST");
    }

    #[test]
    fn env_rate_limit_max_connections_override() {
        let _lock = ENV_LOCK.lock().unwrap();
        clear_rate_limit_env_vars();
        env::set_var("GALE_RATE_LIMIT_MAX_CONNECTIONS_PER_IP", "512");

        let config = Config::load();
        assert_eq!(config.rate_limit.max_connections_per_ip, 512);

        env::remove_var("GALE_RATE_LIMIT_MAX_CONNECTIONS_PER_IP");
    }

    fn clear_cors_env_vars() {
        env::remove_var("GALE_CORS_ENABLED");
        env::remove_var("GALE_CORS_ALLOWED_ORIGINS");
        env::remove_var("GALE_CORS_ALLOWED_METHODS");
        env::remove_var("GALE_CORS_ALLOWED_HEADERS");
        env::remove_var("GALE_CORS_MAX_AGE");
    }

    #[test]
    fn cors_defaults() {
        let _lock = ENV_LOCK.lock().unwrap();
        clear_cors_env_vars();

        let config = Config::load();
        assert!(!config.cors.enabled);
        assert!(config.cors.allowed_origins.is_empty());
        assert_eq!(config.cors.allowed_methods, vec!["GET", "HEAD", "OPTIONS"]);
        assert!(config.cors.allowed_headers.is_empty());
        assert_eq!(config.cors.max_age, 86400);
    }

    #[test]
    fn env_cors_enabled_override() {
        let _lock = ENV_LOCK.lock().unwrap();
        clear_cors_env_vars();
        env::set_var("GALE_CORS_ENABLED", "true");

        let config = Config::load();
        assert!(config.cors.enabled);

        env::remove_var("GALE_CORS_ENABLED");
    }

    #[test]
    fn env_cors_allowed_origins_comma_parsing() {
        let _lock = ENV_LOCK.lock().unwrap();
        clear_cors_env_vars();
        env::set_var("GALE_CORS_ALLOWED_ORIGINS", "https://a.com, https://b.com");

        let config = Config::load();
        assert_eq!(
            config.cors.allowed_origins,
            vec!["https://a.com", "https://b.com"]
        );

        env::remove_var("GALE_CORS_ALLOWED_ORIGINS");
    }

    #[test]
    fn env_cors_max_age_override() {
        let _lock = ENV_LOCK.lock().unwrap();
        clear_cors_env_vars();
        env::set_var("GALE_CORS_MAX_AGE", "3600");

        let config = Config::load();
        assert_eq!(config.cors.max_age, 3600);

        env::remove_var("GALE_CORS_MAX_AGE");
    }

    #[test]
    fn health_endpoint_defaults() {
        let _lock = ENV_LOCK.lock().unwrap();
        clear_server_env_vars();

        let config = Config::load();
        assert_eq!(config.health_endpoint, "/health");
        assert_eq!(config.shutdown_timeout_secs, 10);
    }

    #[test]
    fn env_health_endpoint_override() {
        let _lock = ENV_LOCK.lock().unwrap();
        clear_server_env_vars();
        env::set_var("GALE_HEALTH_ENDPOINT", "/healthz");

        let config = Config::load();
        assert_eq!(config.health_endpoint, "/healthz");

        env::remove_var("GALE_HEALTH_ENDPOINT");
    }

    #[test]
    fn env_health_endpoint_disabled() {
        let _lock = ENV_LOCK.lock().unwrap();
        clear_server_env_vars();
        env::set_var("GALE_HEALTH_ENDPOINT", "");

        let config = Config::load();
        assert!(config.health_endpoint.is_empty());

        env::remove_var("GALE_HEALTH_ENDPOINT");
    }

    #[test]
    fn env_shutdown_timeout_override() {
        let _lock = ENV_LOCK.lock().unwrap();
        clear_server_env_vars();
        env::set_var("GALE_SHUTDOWN_TIMEOUT_SECS", "30");

        let config = Config::load();
        assert_eq!(config.shutdown_timeout_secs, 30);

        env::remove_var("GALE_SHUTDOWN_TIMEOUT_SECS");
    }
}
