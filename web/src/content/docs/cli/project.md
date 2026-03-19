# CLI — Project Commands

The `gale` CLI provides 16 commands. This page covers the core development workflow.

## `gale new <name>`

Interactive project scaffolding.

```bash
gale new my-app
```

Prompts for Tailwind CSS, database adapter (Postgres/SQLite/none), and auth strategy (Session/JWT/none). Generates `galex.toml`, layout, home page, CSS, `.gitignore`, and runs `npm install` if Tailwind is enabled.

## `gale dev`

Development server with hot reload.

```bash
gale dev
gale dev --port 4000
```

Runs a reverse proxy with WebSocket-based hot reload. File changes are debounced at 50ms and classified:

- **GX modified** — incremental rebuild (parse, check, codegen, cargo build, restart)
- **GX structural** — full rebuild
- **CSS changed** — CSS-only hot reload
- **Asset changed** — copy and restart
- **Config changed** — full restart

Injects an error overlay if compilation fails (12 runtime diagnostics, GX1900–GX1911).

## `gale build`

Compile for production.

```bash
gale build --release
gale build --release --docker
```

9-step pipeline: route discovery, parse, type-check, codegen (Rust + JS + CSS), copy assets, Tailwind CSS, minify JS + hash assets, `cargo build --release`, assemble `dist/`.

Flags: `--app-dir` (default: `app`), `--output-dir` (default: `gale_build`), `--name` (default: `gale_app`), `--release`, `--docker`.

## `gale serve`

Run the production binary.

```bash
gale serve
gale serve --port 8080
```

Finds the binary in `dist/`, launches it with port and root args, forwards stdio.

## `gale check`

Type-check without building.

```bash
gale check
```

Runs route discovery, parsing, and type checking. Reports diagnostics with file locations and GX error codes.
