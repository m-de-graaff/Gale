<p align="center">
  <br/>
  <strong>GaleX</strong>
  <br/>
  <em>A type-safe, reactive web framework that compiles to Rust.</em>
  <br/><br/>
  <code>Single binary &middot; SSR by default &middot; Fine-grained reactivity &middot; &lt; 3KB client runtime</code>
</p>

---

## What is GaleX?

GaleX is a full-stack web framework with its own language (`.gx` files) that compiles to a native Rust server binary. You write components with HTML templates, reactive state, typed server actions, and form validation in a single file. The compiler generates an Axum/Tokio server with SSR, hydration, and a sub-3KB client runtime.

```
app/page.gx  -->  gale build  -->  single binary (< 10MB)
```

No Node.js. No bundler config. No runtime framework. Just a compiled binary that serves your app.

## Quick Start

```bash
# Install (requires Rust toolchain)
cargo install galex

# Create a new project
gale new my-app
cd my-app

# Start the dev server (hot reload, error overlay)
gale dev

# Build for production
gale build --release

# Run the production binary
gale serve
```

## Language Overview

A `.gx` file contains components, server logic, and type declarations in a single syntax:

```gx
// Form validation with typed fields
guard ContactForm {
  name: string  @min(1) @max(100)
  email: string @email
}

// Server-side mutation
action submitContact(form: ContactForm) {
  // Runs on the server — has access to DB, env, etc.
  db.insert("contacts", form)
  return { success: true }
}

// Page component with reactive state
out ui ContactPage {
  head {
    title: "Contact Us"
  }

  signal submitted = false

  <main>
    when !submitted.get() {
      <form action={submitContact}>
        <input type="text" name="name" placeholder="Name" />
        <input type="email" name="email" placeholder="Email" />
        <button type="submit">"Send"</button>
      </form>
    }
    when submitted.get() {
      <p>"Thank you for reaching out!"</p>
    }
  </main>
}
```

### Key Features

| Feature | Description |
|---------|-------------|
| **Components** | `out ui Name { }` with HTML templates, reactive code, and `<head>` metadata |
| **Layouts** | `out layout Name { }` with `slot` for page injection |
| **Reactivity** | `signal`, `derive`, `effect`, `watch`, `batch` — fine-grained, no virtual DOM |
| **Guards** | Typed form validation: `guard Name { field: type @validator }` |
| **Actions** | Server mutations: `action name(input: Guard) { }` |
| **Channels** | WebSocket communication: `channel Name { }` |
| **Queries** | Reactive data fetching with caching, retries, stale-time |
| **Stores** | Shared reactive state across components |
| **API routes** | REST endpoints: `out api Resource { get() { } post() { } }` |
| **Middleware** | Request/response interceptors: `middleware Name for /path { }` |
| **Env** | Typed environment variables with compile-time validation |

### Template Directives

```gx
// Conditional rendering
when condition { <p>"Visible"</p> }

// List rendering
each item in items { <li>{item.name}</li> }

// Two-way binding
<input bind:value={name} />

// Event handling
<button on:click={handler}>"Click"</button>

// CSS transitions
<div transition:fade>"Animated"</div>

// Conditional classes
<div class:active={isActive}>"Tab"</div>
```

### Type System

```gx
// Primitives
let x: int = 42
let y: float = 3.14
let s: string = "hello"
let b: bool = true

// Optional types
let name: string? = null

// Arrays
let items: string[] = ["a", "b", "c"]

// Enums (shared between client and server)
shared {
  enum Status { Active, Inactive, Pending }
}

// Boundary blocks
server { /* server-only code */ }
client { /* client-only code */ }
shared { /* available on both sides */ }
```

## Project Structure

```
my-app/
  galex.toml          # Project configuration
  app/
    layout.gx         # Root layout (wraps all pages)
    page.gx           # Home page (route: /)
    about/
      page.gx         # About page (route: /about)
    blog/
      [slug]/
        page.gx       # Dynamic route (route: /blog/:slug)
    guard.gx          # Shared guards
    middleware.gx      # Route middleware
  styles/
    global.css         # Tailwind CSS entry point
  public/
    favicon.ico        # Static assets (served as-is)
```

Routing is file-based. `app/page.gx` maps to `/`, `app/about/page.gx` maps to `/about`, and `app/blog/[slug]/page.gx` maps to `/blog/:slug`.

## CLI Commands

```bash
gale new [name]           # Create a new project
gale dev                  # Dev server with hot reload
gale build [--release]    # Compile to Rust binary
gale serve                # Run production build
gale check                # Type-check without building
gale fmt [--check]        # Format .gx files
gale lint                 # Static analysis
gale test [--filter]      # Run test blocks
```

## Configuration

```toml
# galex.toml

[project]
name = "my-app"

[tailwind]
enabled = true

[database]
adapter = "postgres"      # or "sqlite"

[auth]
strategy = "session"      # or "jwt"
```

## Production Build

```bash
# Build optimized binary with asset hashing
gale build --release

# Output:
# dist/
#   my-app(.exe)          # Single binary (< 10MB)
#   public/
#     _gale/
#       runtime.a1b2c3.js # Content-hashed assets
#       styles.d4e5f6.css
#     favicon.ico          # User assets

# Run it
gale serve

# Or with Docker
gale build --release --docker
docker build -t my-app dist/
docker run -p 8080:8080 my-app
```

The production binary contains the HTTP server, SSR renderer, server actions, API endpoints, WebSocket handlers, and static file serving. Configuration is read from `gale.toml` and environment variables at runtime (no rebuild needed).

## Architecture

```
.gx source files
  |
  v
Parser --> Type Checker --> Code Generator
  |              |               |
  |              |               +--> Rust server code (Axum handlers, SSR)
  |              |               +--> JavaScript (hydration, reactivity)
  |              |               +--> CSS (Tailwind, transitions)
  |              |
  v              v
 AST        Type errors
                 |
                 v
           cargo build --release
                 |
                 v
         Single native binary
```

**Runtime stack**: Axum 0.8 + Tokio (async I/O) + tower-http (middleware) + tracing (logging)

**Client runtime**: < 3KB gzipped. Fine-grained reactivity with direct DOM mutations. No virtual DOM, no diffing, no framework overhead.

## Examples

See [`examples/`](examples/) for complete working projects:

- **test_app** — Minimal two-page app (pipeline verification)

## License

MIT/Apache-2.0.

---

<p align="center"><em>Write once, compile everywhere. Powered by Rust.</em></p>
