#!/usr/bin/env sh
# Install the Gale Zed extension from this local bundle.
#
# Usage (from inside the extracted gale-zed/ directory):
#   sh install-zed.sh
#
# What this does:
#   1. Copies this extension directory into Zed's extensions folder
#   2. Prints a reminder to reload Zed
#
# Zed will pick up the extension on next launch, or via:
#   Cmd/Ctrl+Shift+P → "zed: reload extensions"
set -eu

EXTENSION_ID="gale"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Resolve Zed extensions directory per platform
case "$(uname -s)" in
  Darwin)
    ZED_EXT_DIR="$HOME/Library/Application Support/Zed/extensions/installed/$EXTENSION_ID"
    ;;
  Linux)
    ZED_EXT_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/zed/extensions/installed/$EXTENSION_ID"
    ;;
  *)
    echo "Unsupported OS. Please install manually." >&2
    exit 1
    ;;
esac

echo "Installing Gale Zed extension to:"
echo "  $ZED_EXT_DIR"
echo ""

mkdir -p "$ZED_EXT_DIR"
cp -r "$SCRIPT_DIR/"* "$ZED_EXT_DIR/"

echo "Done. Reload Zed to activate the extension."
echo ""
echo "The LSP (gale-lsp) will be downloaded automatically by Zed from GitHub"
echo "Releases the first time you open a .gx file."
echo ""
echo "If gale-lsp is already on your PATH, Zed will use it directly."
echo "To install it manually, download gale-lsp-<platform>.tar.gz from:"
echo "  https://github.com/m-de-graaff/Gale/releases/latest"
