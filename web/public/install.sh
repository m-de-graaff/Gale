#!/usr/bin/env sh
# Gale SDK installer — macOS and Linux
# Usage: curl -fsSL https://gale.dev/install.sh | sh
#
# Installs:
#   gale      — the Gale CLI (new, dev, build, check, lint, fmt, test)
#   gale-lsp  — the Gale language server (used by VS Code and Zed)
#
# Install directory (in preference order):
#   1. $GALE_INSTALL_DIR  if set
#   2. ~/.local/bin       if on PATH
#   3. ~/.gale/bin        (added to PATH in shell rc)
#
# Supports: macOS (arm64, x86_64), Linux (x86_64, aarch64)
# Requires: curl or wget, tar
set -eu

REPO="m-de-graaff/Gale"
RELEASES="https://github.com/$REPO/releases"

# ── Colours ──────────────────────────────────────────────────────────────────
tty_bold='' tty_reset='' tty_green='' tty_cyan='' tty_yellow='' tty_red=''
if [ -t 1 ]; then
  tty_bold='\033[1m'
  tty_reset='\033[0m'
  tty_green='\033[32m'
  tty_cyan='\033[36m'
  tty_yellow='\033[33m'
  tty_red='\033[31m'
fi

info()    { printf "${tty_cyan}  →${tty_reset} %s\n" "$*"; }
success() { printf "${tty_green}  ✓${tty_reset} %s\n" "$*"; }
warn()    { printf "${tty_yellow}  ⚠${tty_reset} %s\n" "$*"; }
abort()   { printf "${tty_red}  ✗ Error:${tty_reset} %s\n" "$*"; exit 1; }
title()   { printf "\n${tty_bold}%s${tty_reset}\n\n" "$*"; }

# ── OS / arch detection ───────────────────────────────────────────────────────
os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
  Darwin) platform="macos" ;;
  Linux)  platform="linux" ;;
  *)      abort "Unsupported OS: $os. Install with 'cargo install galex' instead." ;;
esac

case "$arch" in
  x86_64|amd64)  cpu="x86_64" ;;
  aarch64|arm64) cpu="aarch64" ;;
  *)             abort "Unsupported architecture: $arch. Install with 'cargo install galex' instead." ;;
esac

# Linux/x86_64 uses musl for zero runtime deps
if [ "$platform" = "linux" ] && [ "$cpu" = "x86_64" ]; then
  sdk_suffix="linux-x86_64"
elif [ "$platform" = "linux" ] && [ "$cpu" = "aarch64" ]; then
  sdk_suffix="linux-aarch64"
elif [ "$platform" = "macos" ] && [ "$cpu" = "aarch64" ]; then
  sdk_suffix="macos-aarch64"
elif [ "$platform" = "macos" ] && [ "$cpu" = "x86_64" ]; then
  sdk_suffix="macos-x86_64"
fi

archive="gale-sdk-${sdk_suffix}.tar.gz"

# ── Download helper ───────────────────────────────────────────────────────────
download() {
  if command -v curl > /dev/null 2>&1; then
    curl --fail --silent --show-error --location "$1" --output "$2"
  elif command -v wget > /dev/null 2>&1; then
    wget --quiet --output-document="$2" "$1"
  else
    abort "Neither curl nor wget found. Install one and re-run."
  fi
}

# ── Resolve latest release tag ────────────────────────────────────────────────
info "Resolving latest Gale release..."
latest_url="https://github.com/$REPO/releases/latest"
redirect_url="$(curl --silent --head --location --max-redirs 1 --output /dev/null --write-out '%{url_effective}' "$latest_url" 2>/dev/null || echo "")"
if [ -z "$redirect_url" ]; then
  redirect_url="$(wget --server-response --max-redirect=0 --quiet --output-document=/dev/null "$latest_url" 2>&1 | grep 'Location:' | tail -1 | sed 's/.*Location: //' | tr -d '[:space:]')"
fi
tag="$(printf '%s' "$redirect_url" | sed 's|.*/tag/||')"
if [ -z "$tag" ]; then
  warn "Could not resolve latest tag from GitHub. Falling back to 'cargo install galex'."
  fallback_to_cargo
fi
info "Latest release: $tag"

# ── Resolve install dir ───────────────────────────────────────────────────────
if [ -n "${GALE_INSTALL_DIR:-}" ]; then
  install_dir="$GALE_INSTALL_DIR"
elif echo ":$PATH:" | grep -q ":$HOME/.local/bin:"; then
  install_dir="$HOME/.local/bin"
else
  install_dir="$HOME/.gale/bin"
fi

# ── Download ─────────────────────────────────────────────────────────────────
title "Installing Gale SDK ($tag)"
info "Platform: $platform ($cpu)"
info "Archive:  $archive"
info "Installing to: $install_dir"

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

download_url="$RELEASES/download/$tag/$archive"
info "Downloading $download_url..."
download "$download_url" "$tmpdir/$archive"

# Verify checksum if sha256sum is available
checksum_url="$RELEASES/download/$tag/checksums.txt"
if command -v sha256sum > /dev/null 2>&1; then
  info "Verifying checksum..."
  download "$checksum_url" "$tmpdir/checksums.txt" 2>/dev/null || warn "Could not download checksums.txt — skipping verification."
  if [ -f "$tmpdir/checksums.txt" ]; then
    (cd "$tmpdir" && grep "$archive" checksums.txt | sha256sum --check --status) \
      && success "Checksum verified." \
      || warn "Checksum mismatch — proceeding anyway, but verify the release manually."
  fi
fi

# Extract
tar -xzf "$tmpdir/$archive" -C "$tmpdir"

# ── Install ───────────────────────────────────────────────────────────────────
mkdir -p "$install_dir"

for binary in gale gale-lsp; do
  if [ -f "$tmpdir/$binary" ]; then
    install -m 755 "$tmpdir/$binary" "$install_dir/$binary"
    success "Installed $binary → $install_dir/$binary"
  else
    warn "$binary not found in archive — skipping."
  fi
done

# ── PATH setup ───────────────────────────────────────────────────────────────
case ":$PATH:" in
  *":$install_dir:"*) : ;; # already on PATH
  *)
    if [ "$install_dir" = "$HOME/.gale/bin" ]; then
      rc_files="$HOME/.bashrc $HOME/.zshrc $HOME/.profile"
      line="export PATH=\"\$HOME/.gale/bin:\$PATH\""
      for rc in $rc_files; do
        if [ -f "$rc" ] && ! grep -q ".gale/bin" "$rc" 2>/dev/null; then
          printf '\n# Gale SDK\n%s\n' "$line" >> "$rc"
          info "Added PATH to $rc"
        fi
      done
      warn "$install_dir is not on PATH."
      warn "Restart your shell or run: export PATH=\"\$HOME/.gale/bin:\$PATH\""
    fi
    ;;
esac

# ── Done ──────────────────────────────────────────────────────────────────────
printf '\n'
success "Gale SDK installed!"
printf '\n'
info "  gale --version"
info "  gale new my-app && cd my-app && gale dev"
printf '\n'
printf "Editor setup: https://gale.dev/editors/vscode  |  https://gale.dev/editors/zed\n"
printf '\n'

fallback_to_cargo() {
  warn "Falling back to 'cargo install galex'..."
  if command -v cargo > /dev/null 2>&1; then
    cargo install galex
    exit 0
  else
    abort "cargo not found. Install Rust from https://rustup.rs and run 'cargo install galex', or download binaries from $RELEASES"
  fi
}
