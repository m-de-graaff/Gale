# Config Reference

Gale uses two configuration files: `galex.toml` for the compiler/framework and `Gale.toml` for the underlying HTTP server.

## galex.toml

Project-level configuration for the GaleX compiler.

```toml
[project]
name = "my-app"
version = "0.1.0"

[tailwind]
enabled = true
config = "tailwind.config.js"

[database]
adapter = "postgres"       # "postgres", "sqlite", or omit

[auth]
strategy = "session"       # "session", "jwt", or omit

[dependencies]
# package-name = "version"
```

## Gale.toml

Server configuration. All settings have secure defaults — zero config is required. Environment variables (`GALE_*`) override file values.

### Server

```toml
[server]
bind = "0.0.0.0"          # GALE_BIND
port = 8080                # GALE_PORT
root = "./public"          # GALE_ROOT
index = "index.html"       # GALE_INDEX
health_endpoint = "/health" # GALE_HEALTH_ENDPOINT
shutdown_timeout = 10       # GALE_SHUTDOWN_TIMEOUT (seconds)
```

### TLS

```toml
[tls]
enabled = false            # GALE_TLS_ENABLED
cert = ""                  # GALE_TLS_CERT (path to PEM cert)
key = ""                   # GALE_TLS_KEY (path to PEM key)
port = 443                 # GALE_TLS_PORT

[tls.acme]
enabled = false            # GALE_TLS_ACME_ENABLED
domains = []               # GALE_TLS_ACME_DOMAINS (comma-separated)
email = ""                 # GALE_TLS_ACME_EMAIL
cache_dir = "./.gale/acme" # GALE_TLS_ACME_CACHE_DIR
```

TLS uses `rustls` (not OpenSSL). ACME support enables automatic Let's Encrypt certificates with hot-reload (10-second polling). HTTP-to-HTTPS redirect is automatic when TLS is enabled.

### Request limits

```toml
[limits]
max_body = 10485760        # GALE_LIMITS_MAX_BODY (10MB, bytes)
max_uri = 8192             # GALE_LIMITS_MAX_URI (bytes)
max_headers = 100          # GALE_LIMITS_MAX_HEADERS (count)
max_header_size = 8192     # GALE_LIMITS_MAX_HEADER_SIZE (bytes)
request_timeout = 30       # GALE_LIMITS_REQUEST_TIMEOUT (seconds)
read_timeout = 10          # GALE_LIMITS_READ_TIMEOUT (seconds)
write_timeout = 10         # GALE_LIMITS_WRITE_TIMEOUT (seconds)
```

### Rate limiting

```toml
[rate_limit]
enabled = true             # GALE_RATE_LIMIT_ENABLED
requests_per_second = 100  # GALE_RATE_LIMIT_RPS
burst = 50                 # GALE_RATE_LIMIT_BURST
max_connections = 256      # GALE_RATE_LIMIT_MAX_CONNECTIONS
```

Per-IP token bucket rate limiter with burst support. Supports `X-Forwarded-For` for clients behind proxies. Includes a periodic cleanup task for expired entries.

### Caching

```toml
[cache]
html = "no-cache"                # GALE_CACHE_HTML
asset = "public, max-age=31536000, immutable"  # GALE_CACHE_ASSET
default = "public, max-age=3600" # GALE_CACHE_DEFAULT
```

Cache-Control headers per content type. HTML gets `no-cache` (always revalidate), hashed assets get immutable (1 year), everything else gets 1 hour. The server adds `Vary: Accept-Encoding` automatically.

### Compression

```toml
[compression]
enabled = true             # GALE_COMPRESSION_ENABLED
algorithms = ["br", "gzip"] # GALE_COMPRESSION_ALGORITHMS
min_size = 1024            # GALE_COMPRESSION_MIN_SIZE (bytes)
```

Brotli and Gzip via `tower-http`. Compression is skipped for:

- Files smaller than `min_size`
- Already-compressed formats: images (JPEG, PNG, WebP, AVIF, GIF), video (MP4, WebM), audio (MP3, OGG, AAC), fonts (WOFF, WOFF2), archives (ZIP, GZ, BR, ZSTD)

The server also supports pre-compressed files (`.br`, `.gz` variants served automatically via `ServeDir`).

### Security headers

```toml
[security]
csp = "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'"
hsts = "max-age=31536000; includeSubDomains"
x_content_type_options = "nosniff"
x_frame_options = "DENY"
x_xss_protection = "0"
referrer_policy = "strict-origin-when-cross-origin"
permissions_policy = "camera=(), microphone=(), geolocation=()"
server_header = ""         # Empty = no Server header (no version disclosure)
block_dotfiles = true      # GALE_SECURITY_BLOCK_DOTFILES
```

All security headers are enabled by default and individually configurable. The server suppresses the `Server` response header by default to prevent version disclosure.

### CORS

```toml
[cors]
enabled = false            # GALE_CORS_ENABLED
allow_origins = ["*"]      # GALE_CORS_ORIGINS
allow_methods = ["GET", "POST", "PUT", "DELETE", "OPTIONS"]
allow_headers = ["Content-Type", "Authorization"]
max_age = 86400
```

CORS is disabled by default. When enabled, the server adds the appropriate `Access-Control-*` headers.

### Logging

```toml
[logging]
level = "info"             # GALE_LOG_LEVEL (trace, debug, info, warn, error)
format = "clf"             # GALE_LOG_FORMAT ("clf" or "json")
output = "stdout"          # GALE_LOG_OUTPUT ("stdout" or file path)
```

Structured logging via `tracing`. CLF (Common Log Format) for human-readable output, JSON for machine parsing. Static asset requests are suppressed in the log output to reduce noise.

## Configuration precedence

```text
Environment variables  >  Gale.toml  >  Defaults
```

All settings have secure defaults. A `Gale.toml` file is optional. Environment variables always win.
