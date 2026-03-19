# Deploying

GaleX compiles to a single binary. No Node.js, no runtime dependencies.

## Release build

```bash
gale build --release
```

Produces `dist/` with the binary and hashed public assets.

## Environment variables

```bash
GALE_PORT=3000 GALE_LOG_FORMAT=json ./dist/my-app
```

App env vars (from `env { }` blocks) load from `.env` via `dotenvy` with fail-fast validation.

## Docker

```dockerfile
FROM rust:alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /app
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl

FROM scratch
COPY --from=builder /app/dist/ /app/
EXPOSE 8080
ENTRYPOINT ["/app/my-app"]
```

Use `gale build --release --docker` to generate this. Final image under 10MB.

## Health checks

`/health` endpoint returns 200 (configurable via `health_endpoint`).

## systemd

From `deploy/gale.service`:

```ini
[Unit]
Description=Gale static web server
After=network-online.target
Wants=network-online.target

[Service]
Type=exec
ExecStart=/usr/local/bin/gale
WorkingDirectory=/var/www
Restart=on-failure
RestartSec=5
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
LimitNOFILE=65535

[Install]
WantedBy=multi-user.target
```

## Graceful shutdown

Handles `SIGTERM`/`SIGINT` (Unix) or Ctrl+C (Windows). Drains within `shutdown_timeout_secs` (default: 10).
