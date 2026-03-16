<p align="center">
  <br/>
  <strong>Gale</strong>
  <br/>
  <em>A brutally fast, minimal, and secure static web server written in Rust.</em>
  <br/><br/>
  <code>Single binary · Zero runtime dependencies · Sub-5MB · Memory-safe</code>
</p>

---

## What is Gale?

Gale is a high-performance static web server built from scratch in Rust. It serves HTML, CSS, JavaScript, images, fonts, video, and every other common web asset with near-zero overhead. No garbage collector, no interpreter, no VM — just compiled machine code responding to HTTP requests as fast as the kernel allows.

The name comes from a gale-force wind: relentless, fast, and impossible to ignore.

## Why build another web server?

Most existing options fall into two camps:

- **Heavy and configurable** (nginx, Apache) — powerful but massive config surface, written in C/C++ with decades of CVE history, and far more complexity than a static site needs.
- **Convenient but slow** (Node/Python dev servers, Caddy) — great developer experience but runtime overhead, garbage collection pauses, and larger memory footprints.

Gale sits in the gap: **production-grade performance with zero unnecessary complexity.** It does one thing — serve static files — and does it exceptionally well.

## Goals

| Priority | Goal |
|----------|------|
| 🥇 | **Raw speed** — competitive with or faster than nginx for static file serving |
| 🥈 | **Security by default** — hardened against OWASP Top 10 out of the box |
| 🥉 | **Minimal footprint** — single static binary < 5MB, runtime RSS < 20MB |
| 4 | **Zero configuration required** — sensible defaults, optional config file |
| 5 | **Minimal dependencies** — only what's necessary, auditable Cargo.toml |

## Non-goals

- Dynamic content, CGI, PHP, reverse proxying, load balancing
- Plugin or module system
- Compatibility with nginx/Apache config formats

## Platform support

| Platform | Tier | Notes |
|----------|------|-------|
| Linux (x86_64, aarch64) | Primary | Static MUSL builds, Docker |
| macOS (x86_64, Apple Silicon) | Primary | Universal binary via `lipo` |
| Windows (x86_64) | Primary | MSVC toolchain |

## Tech stack

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Language | **Rust** (stable) | Fastest compiled language with memory safety guarantees. No GC. |
| Async runtime | **Tokio** | Industry-standard async runtime. Powers Discord, Cloudflare, AWS. |
| HTTP framework | **Axum** | Built by the Tokio team. Minimal, composable, fastest ergonomic option. |
| Middleware | **tower-http** | Compression, CORS, tracing, static files — all as composable layers. |
| TLS | **rustls** | Pure-Rust TLS. No OpenSSL dependency. Modern cipher suites only. |
| Logging | **tracing** | Structured, async-aware logging with zero-cost when disabled. |

**Total core dependencies: 5 crates.** Everything else is from Rust's standard library.

## Feature set

### Serving
- Static file serving from configurable root directory
- Automatic MIME type detection (~60 common types)
- Directory index (index.html fallback)
- Custom error pages (404, 500, etc.)
- Range requests (HTTP 206 — video/audio streaming, download resumption)
- Pre-compressed file support (.gz, .br sidecars served automatically)

### Performance
- Gzip and Brotli response compression with smart skip (no double-compressing images)
- ETag and Last-Modified conditional responses (304 Not Modified)
- Configurable Cache-Control headers per file type
- HTTP/2 with automatic ALPN negotiation over TLS
- Tokio multi-threaded runtime (scales to all available cores)

### Security
- **Path traversal protection** — canonicalized, jailed paths; dotfile blocking
- **Security headers** — CSP, HSTS, X-Content-Type-Options, X-Frame-Options, Referrer-Policy, Permissions-Policy
- **Request limits** — max body size, URI length, header count/size, timeouts
- **Rate limiting** — per-IP request throttling and connection caps
- **TLS 1.2+ only** — strong cipher suites, no legacy protocol support
- **No information leakage** — generic error pages, no server version header
- **CORS** — configurable origin allowlist (never `*` with credentials)

### Operational
- Graceful shutdown (Unix signals + Windows Ctrl+C — Tokio abstracts both)
- Structured access logging (CLF-compatible)
- Health check endpoint (/health)
- ACME / Let's Encrypt auto-renewal (optional)
- Hot certificate reload without restart

## Threat model

Gale is hardened against the OWASP Top 10:2025 risks that apply to a static file server:

| OWASP 2025 | Risk | Gale mitigation |
|------------|------|-----------------|
| A01 | Broken access control | Path jailing, symlink policy, dotfile blocking |
| A02 | Security misconfiguration | Secure defaults, minimal config surface, no default credentials |
| A03 | Supply chain failures | Minimal deps, `cargo audit` in CI, lockfile pinning |
| A04 | Cryptographic failures | rustls with TLS 1.2+ only, strong ciphers, HSTS |
| A05 | Injection | No dynamic content, no user input processing, no query parsing |
| A09 | Logging failures | Structured logging on every request, configurable levels |
| A10 | Mishandled exceptions | Generic error responses, no stack traces in production |

Risks A06 (Insecure design), A07 (Auth failures), A08 (Integrity failures) are not applicable — Gale has no authentication, no user accounts, and no dynamic data processing.

## Build & deploy

```bash
# Development (all platforms)
cargo run -- --root ./public --port 8080

# Release build
cargo build --release
# Binary at: target/release/gale (Linux/macOS) or target/release/gale.exe (Windows)

# Linux — static MUSL build (zero runtime dependencies)
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl

# macOS — universal binary (Intel + Apple Silicon)
rustup target add aarch64-apple-darwin x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin
lipo -create target/aarch64-apple-darwin/release/gale \
     target/x86_64-apple-darwin/release/gale \
     -output target/release/gale-universal

# Windows — MSVC release build
cargo build --release
# Binary at: target\release\gale.exe

# Docker (Linux only)
docker build -t gale .
# FROM scratch — final image < 10MB
```

## Configuration

Gale works with zero configuration. All settings have secure defaults and can be overridden via environment variables or an optional `gale.toml`:

```toml
[server]
bind = "0.0.0.0"
port = 8080
root = "./public"
index = "index.html"

[tls]
enabled = false
cert = "/path/to/cert.pem"
key = "/path/to/key.pem"
# acme = true  # Auto Let's Encrypt

[limits]
max_body_size = "10MB"
max_uri_length = 8192
request_timeout = "30s"
rate_limit = 100          # req/s per IP
max_connections_per_ip = 256

[cache]
default_max_age = 3600
immutable_extensions = ["js", "css", "woff2", "png", "jpg", "gif", "svg"]
immutable_max_age = 31536000

[compression]
enabled = true
min_size = 1024
algorithms = ["br", "gzip"]

[logging]
level = "info"
format = "clf"             # "clf" | "json"
```

## Performance targets

| Metric | Target | Measured against |
|--------|--------|-----------------|
| Requests/sec (static HTML) | > 100,000 | wrk, 12 threads, 400 connections |
| p99 latency | < 5ms | Under sustained load |
| Memory (RSS) | < 20MB | Under heavy load |
| Binary size | < 5MB | Release + LTO + strip |
| Startup time | < 50ms | Cold start to first request served |

## License

MIT/Apache-2.0.

---

<p align="center"><em>Built with Rust.</em></p>
