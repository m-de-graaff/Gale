# Gale SDK installer — Windows (PowerShell)
# Usage: irm https://gale.dev/install.ps1 | iex
#
# Installs:
#   gale.exe      — the Gale CLI
#   gale-lsp.exe  — the Gale language server (used by VS Code and Zed)
#
# Install directory:
#   $env:GALE_INSTALL_DIR  if set
#   $env:LOCALAPPDATA\Gale\bin  otherwise
#
# Requires: PowerShell 5.1+ or PowerShell 7+, .NET (built-in)
$ErrorActionPreference = "Stop"

$Repo         = "m-de-graaff/Gale"
$ReleasesBase = "https://github.com/$Repo/releases"

function Write-Info    { Write-Host "  -> $args" -ForegroundColor Cyan }
function Write-Success { Write-Host "  OK $args" -ForegroundColor Green }
function Write-Warn    { Write-Host "  !! $args" -ForegroundColor Yellow }
function Write-Abort   { Write-Host "  XX Error: $args" -ForegroundColor Red; exit 1 }

# ── Arch detection ────────────────────────────────────────────────────────────
$arch = $env:PROCESSOR_ARCHITECTURE
if ($arch -ne "AMD64") {
    Write-Abort "Only x86_64 (AMD64) is supported. Detected: $arch`nInstall with 'cargo install galex' instead."
}
$SdkSuffix = "windows-x86_64"
$Archive   = "gale-sdk-$SdkSuffix.zip"

# ── Resolve latest release tag ────────────────────────────────────────────────
Write-Info "Resolving latest Gale release..."
try {
    $LatestUrl = "$ReleasesBase/latest"
    $response  = Invoke-WebRequest -Uri $LatestUrl -MaximumRedirection 0 -ErrorAction SilentlyContinue
    $Tag = $response.Headers.Location -replace ".*/tag/", ""
} catch {
    $Tag = ""
}

if (-not $Tag) {
    try {
        $apiUrl = "https://api.github.com/repos/$Repo/releases/latest"
        $json   = Invoke-RestMethod -Uri $apiUrl -Headers @{Accept = "application/vnd.github+json"}
        $Tag    = $json.tag_name
    } catch {
        $Tag = ""
    }
}

if (-not $Tag) {
    Write-Warn "Could not resolve latest release tag. Falling back to cargo install..."
    Invoke-FallbackToCargo
    exit 0
}
Write-Info "Latest release: $Tag"

# ── Resolve install directory ─────────────────────────────────────────────────
if ($env:GALE_INSTALL_DIR) {
    $InstallDir = $env:GALE_INSTALL_DIR
} else {
    $InstallDir = Join-Path $env:LOCALAPPDATA "Gale\bin"
}

Write-Host "`nInstalling Gale SDK ($Tag)" -ForegroundColor White
Write-Info "Archive:       $Archive"
Write-Info "Installing to: $InstallDir"

# ── Download ──────────────────────────────────────────────────────────────────
$TmpDir     = Join-Path $env:TEMP "gale-install-$(Get-Random)"
$ArchivePath = Join-Path $TmpDir $Archive
New-Item -ItemType Directory -Path $TmpDir -Force | Out-Null

$DownloadUrl = "$ReleasesBase/download/$Tag/$Archive"
Write-Info "Downloading $DownloadUrl..."
try {
    Invoke-WebRequest -Uri $DownloadUrl -OutFile $ArchivePath -UseBasicParsing
} catch {
    Write-Abort "Download failed: $_`nFalling back to cargo install galex"
}

# Verify checksum if sha256sum or certutil is available
$ChecksumUrl = "$ReleasesBase/download/$Tag/checksums.txt"
$ChecksumPath = Join-Path $TmpDir "checksums.txt"
try {
    Invoke-WebRequest -Uri $ChecksumUrl -OutFile $ChecksumPath -UseBasicParsing -ErrorAction SilentlyContinue
    if (Test-Path $ChecksumPath) {
        $expected = (Get-Content $ChecksumPath | Where-Object { $_ -match $Archive }) -replace "\s.*", ""
        if ($expected) {
            $actual = (Get-FileHash $ArchivePath -Algorithm SHA256).Hash.ToLower()
            if ($actual -eq $expected.ToLower()) {
                Write-Success "Checksum verified."
            } else {
                Write-Warn "Checksum mismatch. Proceeding anyway — verify the release manually."
            }
        }
    }
} catch {
    Write-Warn "Could not verify checksum — skipping."
}

# Extract
Write-Info "Extracting..."
Expand-Archive -Path $ArchivePath -DestinationPath $TmpDir -Force

# ── Install ───────────────────────────────────────────────────────────────────
if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

foreach ($binary in @("gale.exe", "gale-lsp.exe")) {
    $src = Join-Path $TmpDir $binary
    if (Test-Path $src) {
        $dst = Join-Path $InstallDir $binary
        Copy-Item -Path $src -Destination $dst -Force
        Write-Success "Installed $binary -> $dst"
    } else {
        Write-Warn "$binary not found in archive — skipping."
    }
}

# Cleanup
Remove-Item -Recurse -Force $TmpDir -ErrorAction SilentlyContinue

# ── PATH setup ────────────────────────────────────────────────────────────────
$currentPath = [System.Environment]::GetEnvironmentVariable("PATH", "User") -split ";"
if ($currentPath -notcontains $InstallDir) {
    $newPath = ($currentPath + $InstallDir) -join ";"
    [System.Environment]::SetEnvironmentVariable("PATH", $newPath, "User")
    Write-Success "Added $InstallDir to User PATH."
    Write-Warn "Restart your terminal for PATH changes to take effect."
} else {
    Write-Info "$InstallDir is already on your PATH."
}

# ── Done ──────────────────────────────────────────────────────────────────────
Write-Host ""
Write-Success "Gale SDK installed!"
Write-Host ""
Write-Info "  gale --version"
Write-Info "  gale new my-app; cd my-app; gale dev"
Write-Host ""
Write-Host "Editor setup: https://gale.dev/editors/vscode  |  https://gale.dev/editors/zed" -ForegroundColor DarkGray
Write-Host ""

function Invoke-FallbackToCargo {
    Write-Warn "Attempting 'cargo install galex'..."
    if (Get-Command cargo -ErrorAction SilentlyContinue) {
        cargo install galex
    } else {
        Write-Abort "cargo not found. Install Rust from https://rustup.rs then run 'cargo install galex', or download binaries from $ReleasesBase"
    }
}
