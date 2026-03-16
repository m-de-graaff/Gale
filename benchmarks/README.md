# Gale Benchmarks

Two types of benchmarks measure Gale's performance:

| Type | Tool | Command | Measures |
|------|------|---------|----------|
| Microbenchmarks | Criterion | `cargo bench` | Individual function latency (ns) |
| Load tests | bombardier/wrk | `./benchmarks/run.sh` | Throughput (req/s), p50/p95/p99 latency, RSS |

## Prerequisites

### Criterion (microbenchmarks)

No extra install — Criterion is a dev-dependency in `Cargo.toml`.

### bombardier (load tests, cross-platform)

```bash
# Go install (any platform)
go install github.com/codesenberg/bombardier@latest

# macOS
brew install bombardier

# Or download binary from:
# https://github.com/codesenberg/bombardier/releases
```

### wrk (optional fallback, Linux/macOS only)

```bash
# Ubuntu/Debian
apt install wrk

# macOS
brew install wrk
```

### nginx (comparison benchmark, Linux/macOS only)

```bash
# Ubuntu/Debian
apt install nginx

# macOS
brew install nginx
```

## Running Benchmarks

### Criterion Microbenchmarks

```bash
# Run all benchmarks
cargo bench

# Run a specific benchmark group
cargo bench -- path_security
cargo bench -- mime_lookup
cargo bench -- compression_decision
cargo bench -- cache_extension
cargo bench -- logging_timestamp
cargo bench -- health_handler

# View HTML reports (generated automatically)
open target/criterion/report/index.html      # macOS
xdg-open target/criterion/report/index.html  # Linux
start target/criterion/report/index.html     # Windows
```

Criterion automatically compares against previous runs and reports regressions.

### Load Tests

```bash
# Linux/macOS
./benchmarks/run.sh

# Windows (PowerShell)
.\benchmarks\run.ps1
```

The script:
1. Builds the release binary
2. Generates test fixtures (1KB, 10KB, 100KB, 1MB)
3. Starts Gale with rate limiting disabled
4. Runs bombardier/wrk against multiple endpoints
5. Measures binary size, startup time, and peak RSS
6. Reports pass/fail against performance targets

### nginx Comparison

```bash
# Linux/macOS only (requires nginx installed)
./benchmarks/compare-nginx.sh
```

Runs identical load tests against both Gale and nginx, producing a side-by-side comparison table with throughput ratios.

## Performance Targets

| Metric | Target | Measured By |
|--------|--------|-------------|
| Throughput (small HTML) | > 100,000 req/s | Load test |
| p99 latency (small HTML) | < 5ms | Load test |
| Peak RSS under load | < 20 MB | Load test |
| Binary size (release) | < 5 MB | Load test |
| Cold startup | < 50ms | Load test |
| vs nginx throughput | 0.8x - 1.2x | Comparison |

## Interpreting Results

### Criterion HTML Reports

After running `cargo bench`, open `target/criterion/report/index.html`. Each benchmark group shows:

- **Violin plot**: Distribution of iteration times
- **Linear regression**: Throughput trend
- **Change detection**: Statistical comparison against previous run (green = improved, red = regressed)

### Load Test Output

bombardier output includes:
- **Reqs/sec**: Average throughput
- **Latency (p50/p95/p99)**: Response time percentiles
- **Transfer/sec**: Data throughput

### Key Considerations

- Run benchmarks on a quiet system (no background load)
- Close browsers and other network-heavy applications
- Run multiple times and compare for consistency
- The first run may be slower due to cold caches
- Rate limiting is disabled during load tests (`GALE_RATE_LIMIT_ENABLED=false`)
