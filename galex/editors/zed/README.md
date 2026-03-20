# Gale for Zed

Syntax highlighting and LSP integration for `.gx` files in [Zed](https://zed.dev).

## Features

| Feature | Source | Status |
|---------|--------|--------|
| Syntax highlighting | Tree-sitter `highlights.scm` | Full coverage |
| Diagnostics (errors, warnings) | `gale-lsp` via LSP | Full coverage |
| Autocomplete (context-aware) | `gale-lsp` via LSP | Full coverage |
| Hover information | `gale-lsp` via LSP | Full coverage |
| Go-to-definition | `gale-lsp` via LSP | Full coverage |
| Find all references | `gale-lsp` via LSP | Full coverage |
| Rename symbol | `gale-lsp` via LSP | Full coverage |
| Code actions / quick fixes | `gale-lsp` via LSP | Full coverage |
| Document formatting | `gale-lsp` via LSP | Full coverage |
| Code folding | Tree-sitter `folds.scm` | Full coverage |
| Auto-indentation | Tree-sitter `indents.scm` | Full coverage |
| Symbol outline / breadcrumbs | Tree-sitter `outlines.scm` + LSP | Full coverage |
| Bracket matching | Tree-sitter `brackets.scm` | Full coverage |
| Semantic tokens | `gale-lsp` via LSP | Full coverage |
| Comment toggling | `config.toml` (`//` and `/* */`) | Full coverage |

## Install

### Option A: Downloadable bundle

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

4. Reload Zed (`Cmd/Ctrl+Shift+P` > `zed: reload extensions`)

### Option B: Dev install (for contributors)

```sh
# Clone the repo
git clone https://github.com/m-de-graaff/Gale.git
cd Gale/galex/editors/zed

# In Zed: Cmd/Ctrl+Shift+P > "zed: install dev extension"
# Select this directory
```

## LSP setup

`gale-lsp` is resolved in this order:

1. If `gale-lsp` is found on your `PATH`, Zed uses it directly.
2. If not on PATH, Zed automatically downloads the latest `gale-lsp` binary from GitHub Releases the first time you open a `.gx` file.

To install `gale-lsp` manually:

```sh
# macOS / Linux
curl -LO https://github.com/m-de-graaff/Gale/releases/latest/download/gale-lsp-aarch64-apple-darwin
chmod +x gale-lsp-aarch64-apple-darwin
sudo mv gale-lsp-aarch64-apple-darwin /usr/local/bin/gale-lsp

# Verify
gale-lsp --version
```

## Zed settings

Configure GaleX behavior in `~/.config/zed/settings.json`:

```jsonc
{
  // Language-specific settings for .gx files
  "languages": {
    "GaleX": {
      "tab_size": 2,
      "hard_tabs": false,
      "format_on_save": "on",
      "formatter": "language_server"
    }
  },

  // LSP binary configuration
  "lsp": {
    "gale-lsp": {
      // Override the binary path (if not on PATH)
      "binary": {
        "path": "/usr/local/bin/gale-lsp"
      },
      // Pass settings to the language server
      "settings": {
        "diagnostics": {
          "enabled": true
        },
        "formatting": {
          "enabled": true
        }
      }
    }
  }
}
```

## Troubleshooting

**No syntax highlighting** — Zed compiles the Tree-sitter grammar on first use. If the grammar fails to compile, check Zed's log panel (`Cmd/Ctrl+Shift+P` > `zed: open log`).

**LSP not connecting** — Open Zed's log panel. You should see `gale-lsp` starting. If it fails, verify `gale-lsp` is on PATH and executable (`gale-lsp --version`).

**Extension not appearing** — Make sure you ran the install script and reloaded extensions. Check that the extension directory exists at the path printed by the install script.

**Formatting not working** — Ensure `"formatter": "language_server"` is set in your Zed settings for the GaleX language.

## Architecture

```
editors/zed/
├── extension.toml          # Extension manifest
├── src/lib.rs              # WASM extension (LSP binary management)
├── languages/gale/
│   ├── config.toml         # Language metadata (brackets, comments, tab size)
│   ├── highlights.scm      # Syntax highlighting queries
│   ├── folds.scm           # Code folding queries
│   ├── indents.scm         # Auto-indentation queries
│   ├── outlines.scm        # Symbol outline / breadcrumbs queries
│   ├── brackets.scm        # Bracket matching queries
│   ├── injections.scm      # Embedded language injection queries
│   └── overrides.scm       # Semantic token override scopes
├── grammars/gale/          # Compiled tree-sitter grammar (generated)
├── install-zed.sh          # macOS/Linux install script
├── install-zed.ps1         # Windows install script
└── README.md               # This file
```

The Tree-sitter grammar source is at `tree-sitter-gale/grammar.js` in the repository root. It is compiled and bundled into the extension at release time.
