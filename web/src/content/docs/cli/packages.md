# CLI — Packages & Maintenance

## Package management

### `gale add <package>`

```bash
gale add my-package
```

Fetches metadata from the registry, downloads tarball, verifies SHA-256, extracts to `gale_modules/`, updates `galex.toml` and `gale.lock`.

### `gale remove <package>`

```bash
gale remove my-package
```

Removes from config, lockfile, and `gale_modules/`.

### `gale update [package]`

```bash
gale update           # All packages
gale update my-pkg    # Specific package
```

### `gale search <query>`

```bash
gale search auth
```

Queries the registry search API and prints results.

### `gale publish`

```bash
gale publish
```

Reads `gale-package.toml`, validates semver, packs tarball, computes SHA-256, uploads with auth token.

### `gale login`

```bash
gale login
```

Prompts for token, stores to `~/.gale/credentials`.

## Maintenance

### `gale self-update`

```bash
gale self-update
```

Queries GitHub Releases API, compares semver, downloads platform-specific archive, replaces binary in-place. Also updates `gale-lsp` if present.

### `gale editor install <editor>`

```bash
gale editor install vscode
gale editor install zed
```

Downloads extension from GitHub Releases. For VS Code, runs `code --install-extension`. For Zed, uses platform-specific install scripts.
