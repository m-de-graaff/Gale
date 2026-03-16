# ─────────────────────────────────────────────────
# Gale — Multi-stage Dockerfile
# Final image: FROM scratch, < 10MB, zero attack surface
# ─────────────────────────────────────────────────

# Stage 1: Build static binary
FROM rust:1.83-alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src/ src/

RUN cargo build --release --target x86_64-unknown-linux-musl

# Stage 2: Bare minimum runtime
FROM scratch

# Copy the single static binary
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/gale /gale

# Copy default public directory (override with volume mount)
COPY public/ /public/

# Metadata
LABEL org.opencontainers.image.title="Gale"
LABEL org.opencontainers.image.description="Fast static web server"

EXPOSE 8080
EXPOSE 443

ENTRYPOINT ["/gale"]
CMD ["--root", "/public", "--port", "8080"]
