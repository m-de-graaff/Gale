# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Gale is a static web server written in Rust using Axum/Tokio. It serves static files with production-grade performance, security hardening (OWASP Top 10), and minimal footprint (single binary < 5MB, RSS < 20MB). No dynamic content, no plugins, no reverse proxying.

**Current state:** Specification phase. Cargo.toml, Gale.toml config, Dockerfile, README, and STRUCTURE.md exist but no source code has been written yet. Implementation follows the phased plan in STRUCTURE.md.

## Build & Run Commands

```bash
# Development (all platforms)
cargo run -- --root ./public --port 8080

# Release build (optimized, stripped, ~4MB)
cargo build --release
# Output: target/release/gale (Linux/macOS) or target/release/gale.exe (Windows)

# Linux — static MUSL build (zero runtime deps)
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl

# macOS — universal binary
rustup target add aarch64-apple-darwin x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin
lipo -create target/aarch64-apple-darwin/release/gale \
     target/x86_64-apple-darwin/release/gale \
     -output target/release/gale-universal

# Windows — MSVC release build
cargo build --release

# Run tests
cargo test

# Run a single test
cargo test test_name

# Run integration tests only
cargo test --test integration

# Run benchmarks (requires criterion)
cargo bench

# Lint
cargo clippy -- -D warnings

# Docker (Linux only)
docker build -t gale .
```

## Architecture

### Implementation Phases (from STRUCTURE.md)

| Phase | Scope | Files |
|-------|-------|-------|
| 1 | Core serving | main.rs, config.rs, server.rs, static_files.rs, mime_types.rs, error.rs, platform.rs |
| 2 | Security | security/{mod,path,headers,limits}.rs |
| 3 | Performance | compression.rs, cache.rs |
| 4 | TLS | tls.rs (rustls, ACME) |
| 5 | Observability | logging.rs, rate_limit.rs |
| 7 | Testing | tests/{path_traversal,headers,compression,range,cache,integration}.rs, benches/throughput.rs |

### Middleware Stack Order (tower-http layers on Axum router)

Rate limiting -> Request limits -> Path security -> Security headers -> Compression -> Caching -> Static file serving -> Logging (tracing)

### Configuration Precedence

Environment variables (`GALE_PORT`, `GALE_ROOT`, etc.) override `gale.toml` values. All settings have secure defaults — zero config required. See `Gale.toml` for the complete reference with all defaults documented.

### Key Dependencies (5 core crates only)

- **axum 0.8** — HTTP framework (by Tokio team)
- **tokio 1** (full) — Async runtime
- **tower-http 0.6** — Middleware (fs, compression, headers, trace, cors)
- **tracing 0.1** — Structured logging
- **tracing-subscriber 0.3** — Log formatting (CLF/JSON)
- TLS deps (axum-server, rustls, rustls-pemfile) are commented out until Phase 4

### Release Profile

LTO enabled, single codegen unit, stripped, panic=abort, opt-level=3. This is critical for the < 5MB binary target.

## Design Constraints

- Cross-platform: Linux, macOS, and Windows are all primary targets.
- No dynamic content, CGI, reverse proxying, or plugin system.
- Security headers are on by default (CSP, HSTS, X-Frame-Options, etc.).
- Dotfiles are blocked by default (.git, .env, etc.).
- Compression skips already-compressed formats (images, video, woff2, archives).
- Performance targets: >100k req/s, <5ms p99 latency, <50ms cold start.

### Platform-Specific Implementation Notes

- **Hidden file detection:** `platform.rs` provides `is_hidden()` — checks `.`-prefix on all platforms, additionally checks the Windows hidden file attribute on Windows.
- **Graceful shutdown:** `platform::shutdown_signal()` abstracts Unix SIGTERM/SIGINT and Windows Ctrl+C. Tokio handles both via `tokio::signal`.
- **Symlinks:** `std::fs::metadata` follows symlinks on all platforms. Symlink jail policy applies uniformly.
- **Path handling:** Use `std::path::Path` throughout — Rust normalizes separators. Forward slashes work in config on all platforms.
- **Windows-specific attack vectors:** `security/path.rs` must handle ADS (`:stream`), reserved device names (CON, PRN, NUL, etc.), and UNC paths (`\\?\`) via `#[cfg(windows)]` blocks.
- **CI matrix:** GitHub Actions runs tests on `ubuntu-latest`, `macos-latest`, and `windows-latest`.
