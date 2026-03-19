# Deploying

GaleX compiles to a single binary. No Node.js, no runtime dependencies.

## Release build

```bash
gale build --release
```

This produces a `dist/` directory containing:

- The compiled binary (platform-specific)
- `public/` assets with hashed filenames
- Optionally a Dockerfile (with `--docker` flag)

The binary includes the Gale static server with full middleware stack: security headers, compression, caching, rate limiting, and optional TLS.

## Running in production

```bash
# Direct execution
./dist/my-app --port 8080 --root ./dist/public

# Or use gale serve
gale serve
```

## Environment variables

Override any server setting with environment variables:

```bash
GALE_PORT=3000 \
GALE_ROOT=./dist/public \
GALE_LOG_LEVEL=info \
GALE_LOG_FORMAT=json \
GALE_COMPRESSION_ENABLED=true \
./dist/my-app
```

App-level env vars declared in your `.gx` files (via `env { }`) are loaded from `.env` files via `dotenvy` and validated on startup. Missing required vars cause a fail-fast exit.

See the [Config reference](/docs/config) for all available `GALE_*` variables.

## Docker

The recommended Docker setup uses a multi-stage build:

```dockerfile
# Build stage
FROM rust:alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /app
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl

# Runtime stage
FROM scratch
COPY --from=builder /app/dist/ /app/
EXPOSE 8080
ENTRYPOINT ["/app/my-app"]
```

The `FROM scratch` runtime image contains only the binary and static assets. Final image size is under 10MB.

Use `gale build --release --docker` to generate this Dockerfile automatically.

## Health checks

The server exposes a health endpoint at `/health` (configurable via `GALE_HEALTH_ENDPOINT`). Use this for Kubernetes liveness/readiness probes:

```yaml
livenessProbe:
  httpGet:
    path: /health
    port: 8080
  initialDelaySeconds: 2
  periodSeconds: 10

readinessProbe:
  httpGet:
    path: /health
    port: 8080
  initialDelaySeconds: 1
  periodSeconds: 5
```

The health endpoint returns a 200 status with no body.

## TLS

The server supports TLS via `rustls` with two modes:

**Static certificates:**

```toml
[tls]
enabled = true
cert = "/path/to/cert.pem"
key = "/path/to/key.pem"
port = 443
```

**ACME (Let's Encrypt):**

```toml
[tls]
enabled = true

[tls.acme]
enabled = true
domains = ["example.com", "www.example.com"]
email = "admin@example.com"
cache_dir = "./.gale/acme"
```

When TLS is enabled, the server automatically redirects HTTP to HTTPS. Certificates are hot-reloaded with 10-second polling.

## Graceful shutdown

The server handles `SIGTERM` and `SIGINT` (Unix) or Ctrl+C (Windows) for graceful shutdown. In-flight requests are drained within the configured timeout (default: 10 seconds).

```toml
[server]
shutdown_timeout = 10   # seconds
```

## systemd

A systemd service file is included in the repository at `deploy/gale.service`:

```ini
[Unit]
Description=Gale web server
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/my-app
Restart=always
NoNewPrivileges=true
ProtectSystem=strict

[Install]
WantedBy=multi-user.target
```

## CI/CD

The repository includes GitHub Actions workflows for:

- **CI** (`ci.yml`): test, clippy, and fmt on Linux, macOS, and Windows; MUSL static build; Docker build
- **Release** (`release.yml`): multi-platform SDK bundles (Linux x86_64, macOS aarch64/x86_64, Windows x86_64), editor extensions (VS Code .vsix, Zed archive), SHA-256 checksums, and GitHub Release creation
