#!/usr/bin/env bash
# =============================================================================
# Gale Performance Benchmark Runner
#
# Prerequisites: bombardier (or wrk as fallback)
#   brew install bombardier   # macOS
#   go install github.com/codesenberg/bombardier@latest  # any
#   apt install wrk           # Linux
#
# Usage: ./benchmarks/run.sh
# =============================================================================
set -euo pipefail

PORT=9090
DURATION="30s"
CONNECTIONS=400
BINARY=""
SERVER_PID=""
FIXTURES_DIR=""
RESULTS_DIR="benchmarks/results"
PASS_EMOJI="[PASS]"
FAIL_EMOJI="[FAIL]"

# ---------------------------------------------------------------------------
# Cleanup
# ---------------------------------------------------------------------------
cleanup() {
    if [ -n "$SERVER_PID" ] && kill -0 "$SERVER_PID" 2>/dev/null; then
        kill "$SERVER_PID" 2>/dev/null || true
        wait "$SERVER_PID" 2>/dev/null || true
    fi
    if [ -n "$FIXTURES_DIR" ] && [ -d "$FIXTURES_DIR" ]; then
        rm -rf "$FIXTURES_DIR"
    fi
}
trap cleanup EXIT

# ---------------------------------------------------------------------------
# Detect load test tool
# ---------------------------------------------------------------------------
TOOL=""
if command -v bombardier &>/dev/null; then
    TOOL="bombardier"
elif command -v wrk &>/dev/null; then
    TOOL="wrk"
else
    echo "Error: neither 'bombardier' nor 'wrk' found in PATH."
    echo "Install bombardier: go install github.com/codesenberg/bombardier@latest"
    exit 1
fi
echo "Using load test tool: $TOOL"

# ---------------------------------------------------------------------------
# Platform detection
# ---------------------------------------------------------------------------
OS=$(uname -s)
ARCH=$(uname -m)
echo "Platform: $OS $ARCH"

# ---------------------------------------------------------------------------
# Build release binary
# ---------------------------------------------------------------------------
echo ""
echo "Building release binary..."
cargo build --release 2>&1

if [ "$OS" = "Darwin" ] || [ "$OS" = "Linux" ]; then
    BINARY="target/release/gale"
else
    BINARY="target/release/gale.exe"
fi

if [ ! -f "$BINARY" ]; then
    echo "Error: binary not found at $BINARY"
    exit 1
fi

BINARY_SIZE=$(ls -la "$BINARY" | awk '{print $5}')
BINARY_SIZE_MB=$(echo "scale=1; $BINARY_SIZE / 1048576" | bc)
echo "Binary size: ${BINARY_SIZE_MB} MB"

# ---------------------------------------------------------------------------
# Generate test fixtures
# ---------------------------------------------------------------------------
FIXTURES_DIR=$(mktemp -d)
echo "Generating fixtures in $FIXTURES_DIR ..."

# 1 KB HTML
cat > "$FIXTURES_DIR/small.html" <<'HTMLEOF'
<!DOCTYPE html>
<html lang="en">
<head><meta charset="utf-8"><title>Gale Benchmark</title>
<style>body{font-family:system-ui,sans-serif;max-width:800px;margin:2rem auto;padding:0 1rem}
h1{color:#333}p{line-height:1.6;color:#555}</style></head>
<body><h1>Gale Static Server</h1>
<p>This is a benchmark fixture file used for load testing. It is approximately 1 KB in size
and represents a typical small HTML page served by Gale. The server is designed for
high-throughput static file serving with minimal latency.</p>
<p>Additional paragraph to bring the file closer to 1024 bytes for consistent benchmarking.
Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt
ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation.</p>
</body></html>
HTMLEOF

# 10 KB HTML
{
    echo '<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><title>Medium Page</title></head><body>'
    for i in $(seq 1 50); do
        echo "<p>Paragraph $i: Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.</p>"
    done
    echo '</body></html>'
} > "$FIXTURES_DIR/medium.html"

# 100 KB JS (compressible text)
{
    echo "// Gale benchmark fixture — 100KB JavaScript bundle"
    echo "var BENCHMARK_DATA = {"
    for i in $(seq 1 500); do
        echo "  key_${i}: \"$(head -c 150 /dev/urandom | base64 | tr -d '\n' | head -c 180)\","
    done
    echo "};"
    echo "module.exports = BENCHMARK_DATA;"
} > "$FIXTURES_DIR/bundle.js"

# 1 MB binary (incompressible)
dd if=/dev/urandom of="$FIXTURES_DIR/large.bin" bs=1024 count=1024 2>/dev/null

echo "Fixtures:"
ls -lh "$FIXTURES_DIR/"

# ---------------------------------------------------------------------------
# Start Gale server
# ---------------------------------------------------------------------------
echo ""
echo "Starting Gale on port $PORT ..."
STARTUP_START=$(date +%s%N 2>/dev/null || python3 -c 'import time; print(int(time.time()*1e9))')

GALE_RATE_LIMIT_ENABLED=false \
GALE_PORT=$PORT \
GALE_ROOT="$FIXTURES_DIR" \
GALE_BIND="127.0.0.1" \
GALE_LOGGING_LEVEL="warn" \
"$BINARY" &
SERVER_PID=$!

# Wait for server to be ready
MAX_WAIT=50  # 5 seconds
for i in $(seq 1 $MAX_WAIT); do
    if curl -sf "http://127.0.0.1:$PORT/health" >/dev/null 2>&1; then
        break
    fi
    if ! kill -0 "$SERVER_PID" 2>/dev/null; then
        echo "Error: server process exited unexpectedly"
        exit 1
    fi
    sleep 0.1
done

STARTUP_END=$(date +%s%N 2>/dev/null || python3 -c 'import time; print(int(time.time()*1e9))')

if ! curl -sf "http://127.0.0.1:$PORT/health" >/dev/null 2>&1; then
    echo "Error: server failed to start within 5 seconds"
    exit 1
fi

STARTUP_MS=$(( (STARTUP_END - STARTUP_START) / 1000000 ))
echo "Server ready (startup: ${STARTUP_MS}ms, PID: $SERVER_PID)"

# ---------------------------------------------------------------------------
# Load test helper
# ---------------------------------------------------------------------------
mkdir -p "$RESULTS_DIR"

run_bombardier() {
    local name="$1"
    local url="$2"
    local extra_headers="${3:-}"

    echo "  Testing: $name ..."

    local args=(-c "$CONNECTIONS" -d "$DURATION" -l --print=result)
    if [ -n "$extra_headers" ]; then
        args+=(-H "$extra_headers")
    fi

    bombardier "${args[@]}" "$url" 2>&1 | tee "$RESULTS_DIR/${name}.txt"
}

run_wrk() {
    local name="$1"
    local url="$2"
    local extra_headers="${3:-}"

    echo "  Testing: $name ..."

    local args=(-c "$CONNECTIONS" -d "${DURATION%s}" -t "$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 4)" --latency)
    if [ -n "$extra_headers" ]; then
        args+=(-H "$extra_headers")
    fi

    wrk "${args[@]}" "$url" 2>&1 | tee "$RESULTS_DIR/${name}.txt"
}

run_test() {
    if [ "$TOOL" = "bombardier" ]; then
        run_bombardier "$@"
    else
        run_wrk "$@"
    fi
}

# ---------------------------------------------------------------------------
# Run load tests
# ---------------------------------------------------------------------------
echo ""
echo "Running load tests ($TOOL, ${CONNECTIONS} connections, ${DURATION})..."
echo "============================================================"

BASE="http://127.0.0.1:$PORT"

run_test "small_html_baseline"  "$BASE/small.html"  ""
run_test "small_html_br"        "$BASE/small.html"  "Accept-Encoding: br, gzip"
run_test "medium_html_br"       "$BASE/medium.html"  "Accept-Encoding: br, gzip"
run_test "bundle_js_gzip"       "$BASE/bundle.js"   "Accept-Encoding: gzip"
run_test "large_binary"         "$BASE/large.bin"   ""
run_test "health_endpoint"      "$BASE/health"      ""
run_test "not_found_404"        "$BASE/nonexistent" ""

# ---------------------------------------------------------------------------
# Measure peak RSS
# ---------------------------------------------------------------------------
PEAK_RSS="N/A"
if [ "$OS" = "Linux" ] && [ -f "/proc/$SERVER_PID/status" ]; then
    PEAK_RSS=$(grep VmHWM "/proc/$SERVER_PID/status" 2>/dev/null | awk '{print $2}')
    if [ -n "$PEAK_RSS" ]; then
        PEAK_RSS_MB=$(echo "scale=1; $PEAK_RSS / 1024" | bc)
        PEAK_RSS="${PEAK_RSS_MB} MB"
    fi
elif [ "$OS" = "Darwin" ]; then
    RSS_KB=$(ps -o rss= -p "$SERVER_PID" 2>/dev/null | tr -d ' ')
    if [ -n "$RSS_KB" ]; then
        PEAK_RSS_MB=$(echo "scale=1; $RSS_KB / 1024" | bc)
        PEAK_RSS="${PEAK_RSS_MB} MB"
    fi
fi

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
echo ""
echo "============================================================"
echo "  Gale Performance Benchmark Results"
echo "  Date: $(date +%Y-%m-%d)  |  Platform: $OS $ARCH"
echo "  Binary: ${BINARY_SIZE_MB} MB  |  Peak RSS: ${PEAK_RSS}  |  Startup: ${STARTUP_MS}ms"
echo "============================================================"
echo ""
echo "Raw results saved to: $RESULTS_DIR/"
echo ""

# ---------------------------------------------------------------------------
# Target checks
# ---------------------------------------------------------------------------
check_target() {
    local label="$1"
    local value="$2"
    local threshold="$3"
    local op="$4"  # "lt" or "gt"

    if [ "$op" = "lt" ]; then
        if (( $(echo "$value < $threshold" | bc -l) )); then
            echo "  $PASS_EMOJI $label: $value (target: < $threshold)"
        else
            echo "  $FAIL_EMOJI $label: $value (target: < $threshold)"
        fi
    else
        if (( $(echo "$value > $threshold" | bc -l) )); then
            echo "  $PASS_EMOJI $label: $value (target: > $threshold)"
        else
            echo "  $FAIL_EMOJI $label: $value (target: > $threshold)"
        fi
    fi
}

echo "Targets:"
check_target "Binary < 5 MB" "$BINARY_SIZE_MB" "5" "lt"
check_target "Startup < 50ms" "$STARTUP_MS" "50" "lt"

if [ "$PEAK_RSS" != "N/A" ]; then
    check_target "RSS < 20 MB" "$PEAK_RSS_MB" "20" "lt"
fi

echo ""
echo "Note: Throughput (req/s) and latency (p99) targets require parsing"
echo "tool-specific output. See individual result files in $RESULTS_DIR/"
echo ""
echo "Done."
