# =============================================================================
# Gale Performance Benchmark Runner (Windows / PowerShell)
#
# Prerequisites: bombardier
#   go install github.com/codesenberg/bombardier@latest
#   # Or download from https://github.com/codesenberg/bombardier/releases
#
# Usage: .\benchmarks\run.ps1
# =============================================================================

$ErrorActionPreference = "Stop"

$Port = 9090
$Duration = "30s"
$Connections = 400
$ResultsDir = "benchmarks\results"

# ---------------------------------------------------------------------------
# Check for bombardier
# ---------------------------------------------------------------------------
if (-not (Get-Command bombardier -ErrorAction SilentlyContinue)) {
    Write-Error "bombardier not found in PATH. Install: go install github.com/codesenberg/bombardier@latest"
    exit 1
}
Write-Host "Using load test tool: bombardier"
Write-Host "Platform: Windows $env:PROCESSOR_ARCHITECTURE"

# ---------------------------------------------------------------------------
# Build release binary
# ---------------------------------------------------------------------------
Write-Host ""
Write-Host "Building release binary..."
cargo build --release
if ($LASTEXITCODE -ne 0) { exit 1 }

$Binary = "target\release\gale.exe"
if (-not (Test-Path $Binary)) {
    Write-Error "Binary not found at $Binary"
    exit 1
}

$BinarySize = (Get-Item $Binary).Length
$BinarySizeMB = [math]::Round($BinarySize / 1MB, 1)
Write-Host "Binary size: ${BinarySizeMB} MB"

# ---------------------------------------------------------------------------
# Generate test fixtures
# ---------------------------------------------------------------------------
$FixturesDir = Join-Path ([System.IO.Path]::GetTempPath()) "gale-bench-$(Get-Random)"
New-Item -ItemType Directory -Path $FixturesDir -Force | Out-Null
Write-Host "Generating fixtures in $FixturesDir ..."

# 1 KB HTML
@"
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
"@ | Set-Content -Path (Join-Path $FixturesDir "small.html") -Encoding UTF8

# 10 KB HTML
$mediumContent = "<!DOCTYPE html><html lang=`"en`"><head><meta charset=`"utf-8`"><title>Medium Page</title></head><body>`n"
for ($i = 1; $i -le 50; $i++) {
    $mediumContent += "<p>Paragraph ${i}: Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.</p>`n"
}
$mediumContent += "</body></html>"
$mediumContent | Set-Content -Path (Join-Path $FixturesDir "medium.html") -Encoding UTF8

# 100 KB JS (compressible text)
$jsContent = "// Gale benchmark fixture - 100KB JavaScript bundle`nvar BENCHMARK_DATA = {`n"
$rng = [System.Security.Cryptography.RandomNumberGenerator]::Create()
$buf = New-Object byte[] 135
for ($i = 1; $i -le 500; $i++) {
    $rng.GetBytes($buf)
    $b64 = [Convert]::ToBase64String($buf).Substring(0, 180)
    $jsContent += "  key_${i}: `"$b64`",`n"
}
$jsContent += "};`nmodule.exports = BENCHMARK_DATA;"
$jsContent | Set-Content -Path (Join-Path $FixturesDir "bundle.js") -Encoding UTF8

# 1 MB binary (incompressible)
$binBuf = New-Object byte[] (1024 * 1024)
$rng.GetBytes($binBuf)
[System.IO.File]::WriteAllBytes((Join-Path $FixturesDir "large.bin"), $binBuf)

Write-Host "Fixtures:"
Get-ChildItem $FixturesDir | Format-Table Name, Length -AutoSize

# ---------------------------------------------------------------------------
# Start Gale server
# ---------------------------------------------------------------------------
Write-Host ""
Write-Host "Starting Gale on port $Port ..."

$env:GALE_RATE_LIMIT_ENABLED = "false"
$env:GALE_PORT = $Port
$env:GALE_ROOT = $FixturesDir
$env:GALE_BIND = "127.0.0.1"
$env:GALE_LOGGING_LEVEL = "warn"

$StartupWatch = [System.Diagnostics.Stopwatch]::StartNew()
$ServerProcess = Start-Process -FilePath $Binary -PassThru -WindowStyle Hidden

# Wait for server
$MaxWait = 50
for ($i = 1; $i -le $MaxWait; $i++) {
    try {
        $null = Invoke-WebRequest -Uri "http://127.0.0.1:$Port/health" -UseBasicParsing -TimeoutSec 1
        break
    } catch {
        if ($ServerProcess.HasExited) {
            Write-Error "Server process exited unexpectedly"
            exit 1
        }
        Start-Sleep -Milliseconds 100
    }
}

$StartupWatch.Stop()
$StartupMs = $StartupWatch.ElapsedMilliseconds

try {
    $null = Invoke-WebRequest -Uri "http://127.0.0.1:$Port/health" -UseBasicParsing -TimeoutSec 2
} catch {
    Write-Error "Server failed to start within 5 seconds"
    if (-not $ServerProcess.HasExited) { Stop-Process -Id $ServerProcess.Id -Force }
    Remove-Item -Recurse -Force $FixturesDir
    exit 1
}

Write-Host "Server ready (startup: ${StartupMs}ms, PID: $($ServerProcess.Id))"

# ---------------------------------------------------------------------------
# Load test helper
# ---------------------------------------------------------------------------
New-Item -ItemType Directory -Path $ResultsDir -Force | Out-Null

function Run-Test {
    param(
        [string]$Name,
        [string]$Url,
        [string]$ExtraHeader = ""
    )

    Write-Host "  Testing: $Name ..."

    $args = @("-c", $Connections, "-d", $Duration, "-l", "--print=result")
    if ($ExtraHeader) {
        $args += @("-H", $ExtraHeader)
    }
    $args += $Url

    bombardier @args 2>&1 | Tee-Object -FilePath (Join-Path $ResultsDir "$Name.txt")
}

# ---------------------------------------------------------------------------
# Run load tests
# ---------------------------------------------------------------------------
Write-Host ""
Write-Host "Running load tests (bombardier, $Connections connections, $Duration)..."
Write-Host "============================================================"

$Base = "http://127.0.0.1:$Port"

Run-Test -Name "small_html_baseline" -Url "$Base/small.html"
Run-Test -Name "small_html_br"       -Url "$Base/small.html"  -ExtraHeader "Accept-Encoding: br, gzip"
Run-Test -Name "medium_html_br"      -Url "$Base/medium.html" -ExtraHeader "Accept-Encoding: br, gzip"
Run-Test -Name "bundle_js_gzip"      -Url "$Base/bundle.js"   -ExtraHeader "Accept-Encoding: gzip"
Run-Test -Name "large_binary"        -Url "$Base/large.bin"
Run-Test -Name "health_endpoint"     -Url "$Base/health"
Run-Test -Name "not_found_404"       -Url "$Base/nonexistent"

# ---------------------------------------------------------------------------
# Measure RSS
# ---------------------------------------------------------------------------
$PeakRSS = "N/A"
try {
    $Proc = Get-Process -Id $ServerProcess.Id -ErrorAction Stop
    $PeakRSSMB = [math]::Round($Proc.WorkingSet64 / 1MB, 1)
    $PeakRSS = "$PeakRSSMB MB"
} catch {}

# ---------------------------------------------------------------------------
# Cleanup
# ---------------------------------------------------------------------------
if (-not $ServerProcess.HasExited) {
    Stop-Process -Id $ServerProcess.Id -Force
}
Remove-Item -Recurse -Force $FixturesDir -ErrorAction SilentlyContinue

# Remove env vars
Remove-Item Env:\GALE_RATE_LIMIT_ENABLED -ErrorAction SilentlyContinue
Remove-Item Env:\GALE_PORT -ErrorAction SilentlyContinue
Remove-Item Env:\GALE_ROOT -ErrorAction SilentlyContinue
Remove-Item Env:\GALE_BIND -ErrorAction SilentlyContinue
Remove-Item Env:\GALE_LOGGING_LEVEL -ErrorAction SilentlyContinue

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
Write-Host ""
Write-Host "============================================================"
Write-Host "  Gale Performance Benchmark Results"
Write-Host "  Date: $(Get-Date -Format 'yyyy-MM-dd')  |  Platform: Windows $env:PROCESSOR_ARCHITECTURE"
Write-Host "  Binary: ${BinarySizeMB} MB  |  Peak RSS: ${PeakRSS}  |  Startup: ${StartupMs}ms"
Write-Host "============================================================"
Write-Host ""
Write-Host "Raw results saved to: $ResultsDir\"
Write-Host ""

# Target checks
Write-Host "Targets:"
if ($BinarySizeMB -lt 5)  { Write-Host "  [PASS] Binary < 5 MB: $BinarySizeMB MB" }
else                       { Write-Host "  [FAIL] Binary < 5 MB: $BinarySizeMB MB" }

if ($StartupMs -lt 50)    { Write-Host "  [PASS] Startup < 50ms: ${StartupMs}ms" }
else                       { Write-Host "  [FAIL] Startup < 50ms: ${StartupMs}ms" }

if ($PeakRSS -ne "N/A") {
    if ($PeakRSSMB -lt 20) { Write-Host "  [PASS] RSS < 20 MB: $PeakRSS" }
    else                    { Write-Host "  [FAIL] RSS < 20 MB: $PeakRSS" }
}

Write-Host ""
Write-Host "Note: Throughput (req/s) and latency (p99) targets require parsing"
Write-Host "tool-specific output. See individual result files in $ResultsDir\"
Write-Host ""
Write-Host "Done."
