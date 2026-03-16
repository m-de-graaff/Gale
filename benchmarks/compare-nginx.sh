#!/usr/bin/env bash
# =============================================================================
# Gale vs nginx Side-by-Side Benchmark Comparison
#
# Prerequisites:
#   - bombardier (required)
#   - nginx (installed and in PATH)
#
# Linux/macOS only (nginx doesn't support Windows natively).
#
# Usage: ./benchmarks/compare-nginx.sh
# =============================================================================
set -euo pipefail

GALE_PORT=9090
NGINX_PORT=9091
DURATION="30s"
CONNECTIONS=400
FIXTURES_DIR=""
NGINX_CONF=""
GALE_PID=""
NGINX_PID=""

# ---------------------------------------------------------------------------
# Cleanup
# ---------------------------------------------------------------------------
cleanup() {
    if [ -n "$GALE_PID" ] && kill -0 "$GALE_PID" 2>/dev/null; then
        kill "$GALE_PID" 2>/dev/null || true
        wait "$GALE_PID" 2>/dev/null || true
    fi
    if [ -n "$NGINX_PID" ] && kill -0 "$NGINX_PID" 2>/dev/null; then
        nginx -s stop -c "$NGINX_CONF" 2>/dev/null || kill "$NGINX_PID" 2>/dev/null || true
    fi
    if [ -n "$FIXTURES_DIR" ] && [ -d "$FIXTURES_DIR" ]; then
        rm -rf "$FIXTURES_DIR"
    fi
    if [ -n "$NGINX_CONF" ] && [ -f "$NGINX_CONF" ]; then
        rm -f "$NGINX_CONF"
    fi
}
trap cleanup EXIT

# ---------------------------------------------------------------------------
# Check prerequisites
# ---------------------------------------------------------------------------
if ! command -v bombardier &>/dev/null; then
    echo "Error: bombardier not found. Install: go install github.com/codesenberg/bombardier@latest"
    exit 1
fi

if ! command -v nginx &>/dev/null; then
    echo "Error: nginx not found. Install: apt install nginx / brew install nginx"
    exit 1
fi

echo "Tools: bombardier + nginx"
echo "Platform: $(uname -s) $(uname -m)"

# ---------------------------------------------------------------------------
# Build Gale release binary
# ---------------------------------------------------------------------------
echo ""
echo "Building Gale release binary..."
cargo build --release 2>&1

OS=$(uname -s)
BINARY="target/release/gale"
if [ ! -f "$BINARY" ]; then
    echo "Error: binary not found at $BINARY"
    exit 1
fi

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
        echo "<p>Paragraph $i: Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.</p>"
    done
    echo '</body></html>'
} > "$FIXTURES_DIR/medium.html"

# ---------------------------------------------------------------------------
# Generate nginx.conf
# ---------------------------------------------------------------------------
NGINX_CONF=$(mktemp /tmp/gale-nginx-XXXXXX.conf)
NGINX_TEMP=$(mktemp -d)

cat > "$NGINX_CONF" <<CONFEOF
worker_processes auto;
pid $NGINX_TEMP/nginx.pid;
error_log $NGINX_TEMP/error.log warn;
daemon off;

events {
    worker_connections 1024;
}

http {
    access_log off;

    types {
        text/html html htm;
        text/css css;
        text/javascript js;
        application/json json;
        image/png png;
        image/jpeg jpg jpeg;
    }

    default_type application/octet-stream;
    sendfile on;
    tcp_nopush on;
    tcp_nodelay on;
    keepalive_timeout 65;

    server {
        listen $NGINX_PORT;
        server_name localhost;
        root $FIXTURES_DIR;

        location / {
            try_files \$uri \$uri/ =404;
        }

        location = /health {
            return 200 "OK";
            add_header Content-Type text/plain;
        }
    }
}
CONFEOF

# ---------------------------------------------------------------------------
# Start both servers
# ---------------------------------------------------------------------------
echo ""
echo "Starting Gale on port $GALE_PORT ..."
GALE_RATE_LIMIT_ENABLED=false \
GALE_PORT=$GALE_PORT \
GALE_ROOT="$FIXTURES_DIR" \
GALE_BIND="127.0.0.1" \
GALE_LOGGING_LEVEL="warn" \
"$BINARY" &
GALE_PID=$!

echo "Starting nginx on port $NGINX_PORT ..."
nginx -c "$NGINX_CONF" &
NGINX_PID=$!

# Wait for both
for i in $(seq 1 50); do
    if curl -sf "http://127.0.0.1:$GALE_PORT/health" >/dev/null 2>&1; then break; fi
    sleep 0.1
done

for i in $(seq 1 50); do
    if curl -sf "http://127.0.0.1:$NGINX_PORT/health" >/dev/null 2>&1; then break; fi
    sleep 0.1
done

if ! curl -sf "http://127.0.0.1:$GALE_PORT/health" >/dev/null 2>&1; then
    echo "Error: Gale failed to start"
    exit 1
fi

if ! curl -sf "http://127.0.0.1:$NGINX_PORT/health" >/dev/null 2>&1; then
    echo "Error: nginx failed to start"
    exit 1
fi

echo "Both servers ready."

# ---------------------------------------------------------------------------
# Benchmark helper — returns req/s
# ---------------------------------------------------------------------------
extract_rps() {
    local output="$1"
    echo "$output" | grep -oP 'Reqs/sec\s+\K[\d.]+' 2>/dev/null || \
    echo "$output" | grep -oE '[0-9]+\.[0-9]+ reqs/s' | grep -oE '[0-9]+\.[0-9]+' || \
    echo "N/A"
}

run_bench() {
    local name="$1"
    local port="$2"
    local path="$3"
    local header="${4:-}"

    local args=(-c "$CONNECTIONS" -d "$DURATION" -l --print=result)
    if [ -n "$header" ]; then
        args+=(-H "$header")
    fi

    bombardier "${args[@]}" "http://127.0.0.1:$port$path" 2>&1
}

# ---------------------------------------------------------------------------
# Run comparison
# ---------------------------------------------------------------------------
echo ""
echo "============================================================"
echo "  Gale vs nginx Comparison"
echo "  Date: $(date +%Y-%m-%d)  |  Connections: $CONNECTIONS  |  Duration: $DURATION"
echo "============================================================"
echo ""

printf "%-20s %15s %15s %10s\n" "Test" "Gale (req/s)" "nginx (req/s)" "Ratio"
printf "%-20s %15s %15s %10s\n" "----" "------------" "-------------" "-----"

TESTS=(
    "small.html|/small.html|"
    "medium.html|/medium.html|"
    "health|/health|"
)

for test_spec in "${TESTS[@]}"; do
    IFS='|' read -r name path header <<< "$test_spec"

    gale_output=$(run_bench "$name" "$GALE_PORT" "$path" "$header")
    nginx_output=$(run_bench "$name" "$NGINX_PORT" "$path" "$header")

    gale_rps=$(extract_rps "$gale_output")
    nginx_rps=$(extract_rps "$nginx_output")

    if [ "$gale_rps" != "N/A" ] && [ "$nginx_rps" != "N/A" ] && [ "$nginx_rps" != "0" ]; then
        ratio=$(echo "scale=2; $gale_rps / $nginx_rps" | bc)
        printf "%-20s %15s %15s %9sx\n" "$name" "$gale_rps" "$nginx_rps" "$ratio"
    else
        printf "%-20s %15s %15s %10s\n" "$name" "$gale_rps" "$nginx_rps" "N/A"
    fi
done

echo ""
echo "Target: Gale within 0.8x-1.2x of nginx throughput."
echo ""
echo "Done."
