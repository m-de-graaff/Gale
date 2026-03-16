<!-- This file documents every source file's purpose and phase -->

# Gale — Source File Map

```
gale/
│
├── Cargo.toml                 # Dependencies & release profile
├── Cargo.lock                 # Pinned dependency versions (committed)
├── gale.toml                  # Example config (all defaults documented)
├── Dockerfile                 # Multi-stage: builder → scratch (< 10MB)
├── README.md                  # Project brief, threat model, usage
│
├── src/
│   ├── main.rs                # [Phase 1] Entry point, CLI args, bootstrap
│   ├── config.rs              # [Phase 1] Load gale.toml + env overrides
│   ├── server.rs              # [Phase 1] Axum router, listener, shutdown
│   ├── static_files.rs        # [Phase 1] ServeDir, MIME types, index fallback
│   ├── mime_types.rs          # [Phase 1] Extension → Content-Type map (~60 types)
│   ├── error.rs               # [Phase 1] Error types, custom error pages
│   ├── platform.rs            # [Phase 1] Cross-platform abstractions (hidden files, shutdown signals)
│   ├── security/
│   │   ├── mod.rs             # [Phase 2] Re-exports
│   │   ├── path.rs            # [Phase 2] Path canonicalization, jail, dotfiles
│   │   ├── headers.rs         # [Phase 2] CSP, HSTS, X-Frame-Options, etc.
│   │   └── limits.rs          # [Phase 2] Body size, URI length, timeouts
│   ├── compression.rs         # [Phase 3] Gzip/Brotli middleware, pre-compressed
│   ├── cache.rs               # [Phase 3] ETag, Last-Modified, Cache-Control
│   ├── tls.rs                 # [Phase 4] rustls config, cert loading, ACME
│   ├── logging.rs             # [Phase 5] tracing subscriber, CLF/JSON format
│   └── rate_limit.rs          # [Phase 5] Per-IP token bucket, connection caps
│
├── tests/
│   ├── path_traversal.rs      # [Phase 7] Hundreds of traversal attack variants
│   │                          #           Includes Windows-specific vectors (ADS, reserved names, UNC)
│   ├── headers.rs             # [Phase 7] Verify all security headers present
│   ├── compression.rs         # [Phase 7] Content-Encoding negotiation
│   ├── range.rs               # [Phase 7] HTTP 206 partial content
│   ├── cache.rs               # [Phase 7] ETag, 304, Cache-Control behaviour
│   └── integration.rs         # [Phase 7] Full server lifecycle tests
│                              #           CI runs on Linux, macOS, and Windows
│
├── benches/
│   └── throughput.rs          # [Phase 7] Criterion-based microbenchmarks
│
└── public/                    # Default document root (for development)
    └── index.html             # Placeholder
```
