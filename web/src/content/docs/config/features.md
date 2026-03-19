# Config — Features

## `[cache]`

```toml
[cache]
default_max_age = 3600              # 1 hour
immutable_max_age = 31536000        # 1 year for fingerprinted assets
immutable_extensions = [
    "js", "css", "woff2", "woff", "ttf", "eot",
    "png", "jpg", "jpeg", "gif", "svg", "webp", "avif", "ico",
    "mp4", "webm", "ogg", "wasm",
]
no_cache_extensions = ["html", "htm"]  # Always revalidate
```

The server adds `Vary: Accept-Encoding` automatically.

## `[compression]`

```toml
[compression]
enabled = true
min_size = 1024                     # Don't compress below 1 KB
algorithms = ["br", "gzip"]        # Preference order
pre_compressed = true              # Serve .br/.gz sidecars if present
skip_extensions = [
    "png", "jpg", "jpeg", "gif", "webp", "avif",
    "woff2", "woff",
    "mp4", "webm", "ogg",
    "zip", "gz", "br", "zst",
]
```

Brotli and Gzip via `tower-http`. Already-compressed formats are skipped automatically.

## `[security]`

```toml
[security]
csp = "default-src 'self'"
hsts_max_age = 31536000             # 1 year (only sent over HTTPS)
hsts_include_subdomains = true
x_content_type_options = true       # Sends nosniff
x_frame_options = "DENY"           # DENY | SAMEORIGIN
referrer_policy = "strict-origin-when-cross-origin"
permissions_policy = "camera=(), microphone=(), geolocation=()"
server_header = ""                  # Empty = don't send Server header
block_dotfiles = true               # .git, .env, etc. (+ Windows hidden attribute)
```

All security headers are enabled by default and individually configurable.

## `[cors]`

```toml
[cors]
enabled = false
allowed_origins = []                # e.g. ["https://example.com"]
allowed_methods = ["GET", "HEAD", "OPTIONS"]
allowed_headers = []
max_age = 86400
```

Disabled by default. When enabled, adds `Access-Control-*` headers.

## `[logging]`

```toml
[logging]
level = "info"                      # trace | debug | info | warn | error
format = "clf"                      # "clf" (Common Log Format) | "json"
output = "stdout"                   # "stdout" | "file"
file_path = ""                      # Required if output = "file"
```

Structured logging via `tracing`. Static asset requests are suppressed to reduce noise.
