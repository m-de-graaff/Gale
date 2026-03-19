# Getting Started

> **Gale is in alpha.** The compiler, CLI, and LSP are functional but evolving. Expect rough edges.

## What is Gale?

Gale is a Rust-native web framework built around **GaleX**, a compiled language for writing web applications in `.gx` files. Each `.gx` file contains typed server/client boundaries, guards, actions, and HTML templates. The GaleX compiler transforms these files into a standalone Rust binary (via Axum and Tokio) with server-side rendering, plus client-side JavaScript for interactive elements.

**Gale** is the underlying static web server and LSP. **GaleX** is the language and compiler.

## Install

```bash
# macOS / Linux
curl -fsSL https://get-gale.dev/install.sh | sh

# Windows (PowerShell)
irm https://get-gale.dev/install.ps1 | iex

# Or build from source
cargo install galex
```

## Create a project

```bash
gale new my-app
cd my-app
gale dev
```

`gale new` is interactive — it prompts for Tailwind CSS, database adapter, and auth strategy, then generates a full project structure.

## Project structure

```text
my-app/
  galex.toml          # Compiler configuration
  app/
    layout.gx         # Root layout (must contain <slot/>)
    page.gx            # Home page (route: /)
    about/
      page.gx          # About page (route: /about)
    blog/
      [slug]/
        page.gx        # Dynamic post (route: /blog/:slug)
  public/              # Static assets
  styles/
    app.css            # Tailwind entry point
```

Routes are file-based. Dynamic segments use `[param]`, catch-alls use `[...rest]`.

## Build for production

```bash
gale build --release
```

Output is a single binary in `dist/`. No Node.js runtime needed.

## Next steps

- [Components & Layouts](/docs/reference/components) — pages, layouts, head, slots
- [Guards](/docs/reference/guards) — validation schemas with 28 chain methods
- [Templates](/docs/reference/templates) — directives, conditionals, lists
- [CLI Reference](/docs/cli/project) — all 16 commands
