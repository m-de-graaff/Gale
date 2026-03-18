# Gale for Zed

Syntax highlighting and LSP integration for `.gx` files in [Zed](https://zed.dev).

## What's included

- Syntax highlighting via a Tree-sitter grammar
- Diagnostics, completions, and hover via `gale-lsp`
- Bracket matching, auto-indent, and comment toggling

## Install (downloadable bundle)

1. Download `gale-zed-<version>.zip` from [GitHub Releases](https://github.com/m-de-graaff/Gale/releases/latest)
2. Extract the zip
3. Run the install script from inside the extracted directory:

**macOS / Linux**
```sh
sh install-zed.sh
```

**Windows**
```powershell
.\install-zed.ps1
```

4. Reload Zed (`Cmd/Ctrl+Shift+P` → `zed: reload extensions`)

## LSP setup

`gale-lsp` is resolved in this order:

1. If `gale-lsp` is found on your `PATH`, Zed uses it directly — nothing else needed.
2. If not on PATH, Zed automatically downloads the latest `gale-lsp` binary from GitHub Releases the first time you open a `.gx` file.

To install `gale-lsp` manually, download `gale-lsp-<platform>.tar.gz` from Releases and place the binary somewhere on your PATH:

```sh
# macOS / Linux
tar -xzf gale-lsp-macos-aarch64.tar.gz
sudo mv gale-lsp /usr/local/bin/

# Verify
gale-lsp --version
```

## Manual PATH override (Zed settings)

If `gale-lsp` is at a non-standard path, set it in `~/.config/zed/settings.json`:

```json
{
  "lsp": {
    "gale-lsp": {
      "binary": {
        "path": "/path/to/gale-lsp"
      }
    }
  }
}
```

## Troubleshooting

**No syntax highlighting** — Zed compiles the Tree-sitter grammar on first use. This requires the Zed CLI to be installed. Run `zed --version` to verify. If the grammar fails to compile, check Zed's log panel.

**LSP not connecting** — Open Zed's log panel (`Cmd/Ctrl+Shift+P` → `zed: open log`). You should see `gale-lsp` starting. If it fails, verify `gale-lsp` is on PATH and executable (`gale-lsp --version`).

**Extension not appearing** — Make sure you ran the install script and reloaded extensions. Check that the extension directory exists at the path printed by the install script.
