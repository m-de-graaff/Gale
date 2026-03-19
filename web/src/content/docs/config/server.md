# Config — Server & Limits

All settings have secure defaults. `Gale.toml` is optional. Environment variables override file values.

## `[server]`

```toml
[server]
bind = "0.0.0.0"
port = 8080
root = "./public"
index = "index.html"
error_page_404 = ""             # Path to custom 404.html (empty = built-in)
health_endpoint = "/health"     # Empty = disabled
shutdown_timeout_secs = 10      # Seconds to drain connections
```

## `[tls]`

```toml
[tls]
enabled = false
cert = ""                       # /path/to/fullchain.pem
key = ""                        # /path/to/privkey.pem
redirect_port = 80              # HTTP->HTTPS redirect (0 = disabled)
acme = false                    # Automatic Let's Encrypt
acme_email = ""                 # Required if acme = true
acme_domain = ""                # Required if acme = true
acme_cache_dir = "./acme_cache"
acme_production = false         # false = staging, true = production
```

TLS uses `rustls` (not OpenSSL). When enabled, HTTP-to-HTTPS redirect is automatic. Certificates are hot-reloaded.

## `[limits]`

```toml
[limits]
max_body_size = 10_485_760      # 10 MB
max_uri_length = 8192
max_header_count = 100
max_header_size = 8192          # Per header
request_timeout_secs = 30
read_timeout_secs = 10
write_timeout_secs = 10
```

## `[rate_limit]`

```toml
[rate_limit]
enabled = true
requests_per_second = 100       # Per source IP
max_connections_per_ip = 256
burst = 50                      # Token bucket burst
```

Per-IP token bucket rate limiter. Supports `X-Forwarded-For` for clients behind proxies.

## Configuration precedence

```text
Environment variables  >  Gale.toml  >  Defaults
```
