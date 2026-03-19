# Getting Started

> **Gale is in alpha.** The compiler, CLI, and LSP are functional but evolving. Expect rough edges.

## What is Gale?

Gale is a Rust-native web framework built around **GaleX**, a compiled language for writing web applications in `.gx` files. Each `.gx` file contains typed server/client boundaries, guards, actions, and HTML templates. The GaleX compiler transforms these files into a standalone Rust binary (via Axum and Tokio) with server-side rendering, plus client-side JavaScript for interactive elements.

**Gale** is the underlying static web server and LSP. **GaleX** is the language and compiler.

## Install

The SDK installer puts `gale` (CLI) and `gale-lsp` (language server) on your PATH:

```bash
# macOS / Linux
curl -fsSL https://get-gale.dev/install.sh | sh

# Windows (PowerShell)
irm https://get-gale.dev/install.ps1 | iex

# Or build from source
cargo install galex
```

The SDK installer detects your OS and architecture, downloads from GitHub Releases, verifies the SHA-256 checksum, and patches your shell profile for PATH. No admin rights required.

## Create a project

```bash
gale new my-app
cd my-app
```

`gale new` is interactive. It prompts you to choose:

- **Tailwind CSS** integration (yes/no)
- **Database** adapter (Postgres, SQLite, or none)
- **Auth** strategy (Session, JWT, or none)

It then generates a project with `galex.toml`, a layout, a home page, CSS, `.gitignore`, and runs `npm install` if Tailwind is enabled.

## Project structure

```text
my-app/
  galex.toml          # Compiler and project configuration
  app/
    layout.gx         # Root layout (wraps all pages, must contain <slot/>)
    page.gx            # Home page (route: /)
    about/
      page.gx          # About page (route: /about)
    blog/
      page.gx          # Blog index (route: /blog)
      [slug]/
        page.gx        # Dynamic blog post (route: /blog/:slug)
  public/              # Static assets (copied to output)
  styles/
    app.css            # Global styles (Tailwind entry point)
```

Routes are file-based. `app/page.gx` becomes `/`. `app/about/page.gx` becomes `/about`. Dynamic segments use `[param]` and catch-alls use `[...rest]`.

## Your first page

```text
out ui HomePage() {
  head {
    title: "My App"
    description: "Built with GaleX"
  }

  <main>
    <h1>Welcome</h1>
    <p>This is a GaleX application.</p>
  </main>
}
```

Every page is an `out ui` component. The `head` block sets `<title>` and `<meta>` tags (the compiler warns if title or description are missing via GX1403/GX1404).

## Add a server action

```text
guard ContactForm {
  email: string.trim().email()
  message: string.trim().minLen(10).maxLen(500)
}

server {
  action submit(data: ContactForm) -> string {
    await send_email(data.email, data.message)
    return "Sent"
  }
}
```

Guards define typed validation schemas. The compiler checks that validator methods are compatible with their field types (e.g., `.email()` only works on `string` fields, `.min()` only on `int` or `float`). Actions run on the server and are exposed as RPC endpoints.

## Run the dev server

```bash
gale dev
```

The dev server:

1. Discovers routes from `app/` directory structure
2. Parses and type-checks all `.gx` files
3. Generates a Rust project and compiles it
4. Starts an Axum reverse proxy with WebSocket hot reload
5. Watches for file changes with 50ms debounce
6. Incrementally rebuilds on `.gx`, CSS, asset, or config changes
7. Injects an error overlay into the page if compilation fails

## Build for production

```bash
gale build --release
```

The build pipeline:

1. Route discovery
2. Parse all `.gx` files
3. Type-check the merged program
4. Generate Rust server code + client JavaScript + CSS
5. Copy `public/` assets
6. Run Tailwind CSS CLI (if configured)
7. Minify JavaScript and hash assets for cache-busting
8. Run `cargo build --release` on the generated project
9. Assemble `dist/` with the binary + public assets

The output is a single binary in `dist/`. No Node.js runtime needed in production.

```bash
# Run the production binary
gale serve

# Or run it directly
./dist/my-app --port 8080 --root ./dist/public
```

## Next steps

- [Language Reference](/docs/reference) — every `.gx` construct
- [API Reference](/docs/api) — guards, actions, channels, stores, queries
- [CLI Reference](/docs/cli) — all 18 commands
- [Forms guide](/docs/guides/forms) — guard + form:action + form:guard pattern
