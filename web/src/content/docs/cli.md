# CLI Reference

The `gale` CLI provides 18 commands for the full development lifecycle.

## Project commands

### `gale new <name>`

Interactive project scaffolding.

```bash
gale new my-app
```

Prompts for Tailwind CSS, database adapter (Postgres/SQLite/none), and auth strategy (Session/JWT/none). Generates `galex.toml`, layout, home page, CSS, `.gitignore`, and runs `npm install` if Tailwind is enabled.

### `gale dev`

Start the development server with hot reload.

```bash
gale dev
```

Runs a reverse proxy with WebSocket-based hot reload. File changes are debounced at 50ms and classified into:

- **GX modified** — incremental rebuild (parse, check, codegen, cargo build, restart)
- **GX structural** — full rebuild
- **CSS changed** — CSS-only hot reload
- **Asset changed** — copy and restart
- **Config changed** — full restart

Injects an error overlay into the page if compilation fails (12 runtime error diagnostics, GX1900–GX1911).

### `gale build`

Compile the project for production.

```bash
gale build --release
```

9-step pipeline:

1. Route discovery from `app/` directory
2. Parse all `.gx` files
3. Type-check the merged program
4. Generate Rust + JS + CSS
5. Copy `public/` assets
6. Run Tailwind CSS (if configured)
7. Minify JS and hash assets
8. Run `cargo build --release`
9. Assemble `dist/` directory

Supports `--docker` flag to include a Dockerfile in the output.

### `gale serve`

Run the production binary from `dist/`.

```bash
gale serve
```

Finds the built binary in `dist/`, launches it with port and root arguments, and forwards stdio.

### `gale check`

Type-check without building.

```bash
gale check
```

Runs route discovery, parsing, and type checking. Reports all diagnostics with file locations, line numbers, and GX error codes.

## Quality commands

### `gale lint`

Static analysis with 13 rules.

```bash
gale lint
```

| Rule | Code | Description |
|------|------|-------------|
| Unused signals | GX1700 | Signal declared but never read |
| Unused derives | GX1701 | Derive declared but never read |
| Unused variables | GX1702 | Variable declared but never used |
| Empty blocks | GX1703 | Block with no statements |
| Missing `key` on `each` | GX1705 | List items without identity key |
| Missing `alt` on `img` | GX1706 | Image without alt text |
| Unreachable after return | GX1707 | Code after return statement |
| `console.log` | GX1708 | Debug logging left in code |
| Unnecessary else after return | GX1709 | Else block after return in if |
| Missing label for input | GX1710 | Input without associated label |
| Function too long | GX1711 | Function exceeds line threshold |
| File too long | GX1712 | File exceeds line threshold |
| TODO comments | GX1713 | Unresolved TODO/FIXME comments |

### `gale fmt`

> **Status: Disabled.** The formatter is implemented but intentionally disabled due to known bugs: it drops parenthesized expression grouping and strips all comments. Running `gale fmt` returns an error message.

### `gale test`

> **Status: Partial.** Test discovery works — the compiler finds and parses `test` blocks in `.gx` files. However, the test runner does not yet compile test bodies to Rust. All discovered tests pass vacuously.

```bash
gale test
```

## Package commands

### `gale add <package>`

Install a package from the registry.

```bash
gale add my-package
```

Fetches metadata from the registry API, downloads the tarball, verifies SHA-256 checksum, extracts to `gale_modules/`, and updates `galex.toml` dependencies and `gale.lock`.

### `gale remove <package>`

Remove a package.

```bash
gale remove my-package
```

Removes from config, lockfile, and `gale_modules/` directory.

### `gale update [package]`

Update packages to latest versions.

```bash
gale update           # Update all
gale update my-pkg    # Update specific package
```

### `gale search <query>`

Search the package registry.

```bash
gale search auth
```

Queries the registry search API and prints matching packages.

### `gale publish`

Publish a package to the registry.

```bash
gale publish
```

Reads `gale-package.toml`, validates semver, packs a tarball, computes SHA-256 checksum, and uploads with your auth token.

### `gale login`

Authenticate with the package registry.

```bash
gale login
```

Prompts for your token and stores it to `~/.gale/credentials`.

## Maintenance commands

### `gale self-update`

Update the CLI to the latest version.

```bash
gale self-update
```

Queries GitHub Releases API, compares semver, downloads the platform-specific archive, extracts it, and replaces the binary in-place. Also updates `gale-lsp` if present alongside the CLI.

### `gale editor install <editor>`

Install an editor extension.

```bash
gale editor install vscode
gale editor install zed
```

Downloads the extension from GitHub Releases and installs it using the editor's CLI (`code --install-extension` for VS Code) or platform-specific install scripts for Zed.
