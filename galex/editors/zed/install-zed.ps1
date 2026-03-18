# Install the Gale Zed extension from this local bundle.
#
# Usage (from inside the extracted gale-zed\ directory):
#   .\install-zed.ps1
#
# What this does:
#   1. Copies this extension directory into Zed's extensions folder
#   2. Prints a reminder to reload Zed

$ErrorActionPreference = "Stop"

$ExtensionId = "gale"
$ScriptDir   = Split-Path -Parent $MyInvocation.MyCommand.Path

# Zed extensions directory on Windows
$ZedExtDir = Join-Path $env:APPDATA "Zed\extensions\installed\$ExtensionId"

Write-Host "Installing Gale Zed extension to:"
Write-Host "  $ZedExtDir"
Write-Host ""

if (-not (Test-Path $ZedExtDir)) {
    New-Item -ItemType Directory -Path $ZedExtDir -Force | Out-Null
}

Get-ChildItem -Path $ScriptDir | Copy-Item -Destination $ZedExtDir -Recurse -Force

Write-Host "Done. Reload Zed to activate the extension."
Write-Host ""
Write-Host "The LSP (gale-lsp) will be downloaded automatically by Zed from GitHub"
Write-Host "Releases the first time you open a .gx file."
Write-Host ""
Write-Host "If gale-lsp is already on your PATH, Zed will use it directly."
Write-Host "To install it manually, download gale-lsp-windows-x86_64.zip from:"
Write-Host "  https://github.com/m-de-graaff/Gale/releases/latest"
