# ─────────────────────────────────────────────────
# Gale — Multi-stage Dockerfile
# Final image: FROM scratch, < 10MB, zero attack surface
# Note: This Dockerfile builds a Linux container image.
# For macOS/Windows, build natively with cargo build --release.
# ─────────────────────────────────────────────────

# Stage 1: Build static binary
FROM rust:1.83-alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
# Stub manifests for workspace members not needed in this image,
# so Cargo can parse the workspace without their full source trees.
RUN mkdir -p galex/src gale-registry/src benches && \
    printf '[package]\nname = "galex"\nversion = "0.1.0"\nedition = "2021"\n' > galex/Cargo.toml && \
    printf '[package]\nname = "gale-registry"\nversion = "0.1.0"\nedition = "2021"\n' > gale-registry/Cargo.toml && \
    touch galex/src/main.rs && \
    touch gale-registry/src/main.rs && \
    touch benches/throughput.rs
COPY src/ src/

RUN cargo build --release --target x86_64-unknown-linux-musl -p gale

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

STOPSIGNAL SIGTERM

ENTRYPOINT ["/gale"]
CMD ["--root", "/public", "--port", "8080"]
