export interface InstallTab {
  label: string
  command: string
  note: string
}

export interface FeatureCard {
  eyebrow: string
  title: string
  body: string
}

export interface ExampleCard {
  name: string
  summary: string
  status: 'reference' | 'repo'
  featureTags: string[]
  source?: string
  caveat?: string
  code: string
}

export interface EditorGuide {
  slug: 'vscode' | 'zed'
  title: string
  summary: string
  artifact: string
  installCommand: string
  quickSteps: string[]
  notes: string[]
  troubleshooting: string[]
}

export interface DocSection {
  id: string
  title: string
  body: string
  code?: string
  codeLanguage?: string
  table?: { headers: string[]; rows: string[][] }
}

export interface DocPage {
  path: string
  navLabel: string
  group: string
  title: string
  description: string
  badge: string
  sections: DocSection[]
  related: string[]
}

export const REPO_URL = 'https://github.com/m-de-graaff/Gale'
export const RELEASES_URL = `${REPO_URL}/releases`

export const installTabs: InstallTab[] = [
  {
    label: 'macOS / Linux',
    command: 'curl -fsSL https://get-gale.vercel.app/install.sh | sh',
    note: 'Installs both `gale` and `gale-lsp` into ~/.local/bin.',
  },
  {
    label: 'Windows',
    command: 'irm https://get-gale.vercel.app/install.ps1 | iex',
    note: 'Installs `gale.exe` and `gale-lsp.exe` into %LOCALAPPDATA%\\Gale\\bin.',
  },
  {
    label: 'Cargo',
    command: 'cargo install galex',
    note: 'Fallback install path while package naming converges with the Gale brand.',
  },
]

export const featureCards: FeatureCard[] = [
  {
    eyebrow: 'Native output',
    title: 'Write .gx. Ship a binary.',
    body: 'Gale compiles your application into a Rust-native server. No Node process, no JavaScript runtime tree.',
  },
  {
    eyebrow: 'Typed boundaries',
    title: 'Server and client stay honest.',
    body: 'Guards, actions, env declarations, and boundary analysis keep server-only capabilities from leaking into the browser bundle.',
  },
  {
    eyebrow: 'SSR by default',
    title: 'Fast first paint without ceremony.',
    body: 'Pages render on the server, hydrate a tiny client runtime, and keep the routing model file-based and predictable.',
  },
  {
    eyebrow: 'Operationally small',
    title: 'One CLI, one binary, one deploy.',
    body: 'The same toolchain handles dev, build, check, lint, format, and packaging. One binary ships to production.',
  },
  {
    eyebrow: 'Realtime built in',
    title: 'Channels, actions, API routes.',
    body: 'Not just page rendering. Gale has a model for server mutations, HTTP endpoints, WebSocket channels, and shared types.',
  },
  {
    eyebrow: 'Editor tooling',
    title: 'Language server ships with the stack.',
    body: '`gale-lsp` powers diagnostics in VS Code and Zed. Editor artifacts are downloadable from every GitHub release.',
  },
]

export const canonicalExamples: ExampleCard[] = [
  {
    name: 'Protected Admin Dashboard',
    summary:
      'Auth with login form, middleware, role checks, session-backed actions, and env-driven secrets. The pattern the auth guide is built on.',
    status: 'reference',
    featureTags: ['middleware', 'env', 'actions', 'guards', 'protected routes'],
    code: `env {
  server {
    SESSION_SECRET: string.min(32)
  }
}

guard LoginForm {
  email:    string.email().trim()
  password: string.min(12)
}

server {
  out server action login(form: LoginForm) -> AuthResult {
    let user = auth.verify(form.email, form.password)
    session.start(user.id)
    redirect('/admin')
  }
}

middleware AdminOnly for /admin {
  fn handle(req, next) -> Response {
    when !session.exists() {
      return redirect('/login')
    }
    return next(req)
  }
}`,
  },
  {
    name: 'Orders API + CRUD Console',
    summary:
      '`out api` resources, typed actions, and database queries showing the full data-flow path the docs now optimise for.',
    status: 'reference',
    featureTags: ['out api', 'db access', 'actions', 'guards'],
    code: `guard CreateOrder {
  customerEmail: string.email()
  total:         float.positive()
}

out api Orders {
  get() {
    return db.query('select * from orders order by created_at desc')
  }

  post(form: CreateOrder) {
    return db.insert('orders', form)
  }
}

server {
  out server action archiveOrder(id: int) -> { ok: bool } {
    db.update('orders', { archived: true }, { id })
    return { ok: true }
  }
}`,
  },
  {
    name: 'Realtime Support Inbox',
    summary:
      'A channel-driven inbox with presence, reconnect, authenticated rooms, and durable server history. The pattern the realtime guide is built on.',
    status: 'reference',
    featureTags: ['channels', 'presence', 'actions', 'auth', 'shared types'],
    code: `shared {
  type SupportMessage = {
    author: string
    body:   string
    sentAt: string
  }
}

server {
  channel SupportRoom <-> SupportMessage

  out server action seedHistory() -> SupportMessage[] {
    return db.query('select * from support_messages order by sent_at asc')
  }
}

client {
  signal history = await seedHistory()
  signal room    = channel('SupportRoom')
}`,
  },
  {
    name: 'Checkout Flow',
    summary:
      'Cart mutations, payment env config, guarded forms, and API/webhook boundaries combined into one coherent example.',
    status: 'reference',
    featureTags: ['actions', 'env', 'guards', 'api routes', 'server-only deps'],
    code: `out env {
  server {
    STRIPE_SECRET: string.min(20)
  }
  client {
    GALE_PUBLIC_STRIPE_KEY: string.min(20)
  }
}

guard CheckoutForm {
  email:           string.email()
  shippingAddress: string.min(10)
}

server {
  out server action beginCheckout(form: CheckoutForm) -> CheckoutSession {
    return payments.createSession(form, env.STRIPE_SECRET)
  }
}

out api Webhooks {
  post() {
    return payments.handleWebhook(request.body)
  }
}`,
  },
]

export const repoExamples: ExampleCard[] = [
  {
    name: 'examples/chat',
    summary: 'UI framing and page structure for a chat interface.',
    status: 'repo',
    source: 'examples/chat/app/page.gx',
    caveat:
      'The UI describes channels conceptually but does not wire up a real channel declaration end-to-end yet.',
    featureTags: ['ui shell', 'layout'],
    code: `out ui ChatRoom {
  head { title: 'Chat Room' }

  signal username = 'Anonymous'

  <main>
    <h1>"Chat Room"</h1>
    <p>"Real-time messaging powered by Gale channels."</p>
  </main>
}`,
  },
  {
    name: 'examples/ecommerce',
    summary: 'Multi-page routing, layouts, and a shared enum across a storefront.',
    status: 'repo',
    source: 'examples/ecommerce/app/page.gx',
    caveat:
      'Good for layout and routing demos. Cart and checkout actions are stubbed, not fully wired.',
    featureTags: ['multi-page routing', 'shared enums'],
    code: `shared {
  enum Category { Electronics, Clothing, Books, Home }
}

out ui StorePage {
  head { title: 'Shop' }
  <h1>"Shop"</h1>
}`,
  },
  {
    name: 'examples/dashboard',
    summary: 'Persistent dashboard sidebar shell and admin IA across multiple routes.',
    status: 'repo',
    source: 'examples/dashboard/app/page.gx',
    caveat: 'Useful for navigation structure. Metric values are static placeholders today.',
    featureTags: ['admin IA', 'sidebar layout', 'route groups'],
    code: `out ui DashboardOverview {
  head { title: 'Dashboard' }

  signal activeUsers = 0
  signal totalOrders = 0
}`,
  },
]

export const editorGuides: EditorGuide[] = [
  {
    slug: 'vscode',
    title: 'Gale for VS Code',
    summary:
      'Install a packaged `.vsix` from GitHub Releases and point the extension at the `gale-lsp` binary that ships with the Gale SDK.',
    artifact: 'gale-vscode-<version>.vsix',
    installCommand: 'code --install-extension gale-vscode-<version>.vsix',
    quickSteps: [
      'Install the Gale SDK so `gale` and `gale-lsp` are on your PATH.',
      'Download `gale-vscode-<version>.vsix` from GitHub Releases.',
      'Install it via the Extensions panel ("Install from VSIX…") or with the `code` CLI shown above.',
      'Open a `.gx` file. If diagnostics do not appear, set `gale.lspPath` in settings to the full path of `gale-lsp`.',
    ],
    notes: [
      'The extension provides syntax highlighting, snippets, diagnostics, hover docs, and go-to-definition for `.gx` files.',
      'The downloadable `.vsix` route avoids depending on marketplace publishing while Gale is in early development.',
    ],
    troubleshooting: [
      'No diagnostics: run `gale-lsp --help` in a terminal to confirm the binary is on PATH and executable.',
      'Wrong binary: set `gale.lspPath` to the absolute path of `gale-lsp` if you installed it outside your default PATH.',
      'Old version: delete the existing extension and reinstall from the latest `.vsix` on the Releases page.',
    ],
  },
  {
    slug: 'zed',
    title: 'Gale for Zed',
    summary:
      'Extract the Zed bundle from GitHub Releases, run the included install script, and let it patch the bundled grammar into your Zed extensions directory.',
    artifact: 'gale-zed-<version>.zip',
    installCommand: './install-zed.sh  # or install-zed.ps1 on Windows',
    quickSteps: [
      'Install the Gale SDK so `gale-lsp` is available outside Zed as well.',
      'Download and extract `gale-zed-<version>.zip` from GitHub Releases.',
      'Run `./install-zed.sh` (macOS/Linux) or `./install-zed.ps1` (Windows).',
      'Restart Zed and open a `.gx` file to confirm syntax highlighting and the language server both start.',
    ],
    notes: [
      'The bundle ships a local Tree-sitter grammar folder so installation works without a machine-specific repository path.',
      'The install scripts copy the grammar into Zed\'s extension directory and patch the `extension.toml` automatically.',
    ],
    troubleshooting: [
      'Grammar fails to load: rerun the install script so `extension.toml` is patched with the correct local path.',
      'LSP does not start: run `gale-lsp --help` in a terminal. If that fails, reinstall the Gale SDK.',
      'On Linux, confirm the target directory is `$XDG_DATA_HOME/zed/extensions/installed/gale` or `~/.local/share/zed/extensions/installed/gale`.',
    ],
  },
]

export const downloadMatrix = [
  { kind: 'Gale SDK', name: 'gale-sdk-linux-x86_64.tar.gz', target: 'Linux x86_64', notes: '`gale` + `gale-lsp`' },
  { kind: 'Gale SDK', name: 'gale-sdk-macos-aarch64.tar.gz', target: 'macOS Apple Silicon', notes: '`gale` + `gale-lsp`' },
  { kind: 'Gale SDK', name: 'gale-sdk-macos-x86_64.tar.gz', target: 'macOS Intel', notes: '`gale` + `gale-lsp`' },
  { kind: 'Gale SDK', name: 'gale-sdk-windows-x86_64.zip', target: 'Windows x86_64', notes: '`gale.exe` + `gale-lsp.exe`' },
  { kind: 'VS Code', name: 'gale-vscode-<version>.vsix', target: 'All platforms', notes: 'Manual VSIX install' },
  { kind: 'Zed', name: 'gale-zed-<version>.zip', target: 'All platforms', notes: 'Bundled grammar + install scripts' },
  { kind: 'Checksums', name: 'checksums.txt', target: '—', notes: 'SHA-256 for all release files' },
]

export const docsPages: DocPage[] = [
  {
    path: '/docs/getting-started',
    navLabel: 'Getting Started',
    group: 'Guide',
    title: 'Getting started with Gale',
    description: 'Install the SDK, create your first project, and start with a real server-bound form flow rather than a static counter.',
    badge: 'First app',
    sections: [
      {
        id: 'what-is-gale',
        title: 'What Gale is',
        body: 'Gale is a Rust-native web framework. GaleX is the `.gx` syntax that describes pages, server actions, guards, API routes, and client reactivity in a single source model. The framework compiles `.gx` files into a standalone Rust binary that serves your app with full SSR and a sub-3 KB client runtime.',
      },
      {
        id: 'install',
        title: 'Install Gale',
        body: 'The recommended path is the Gale SDK installer — it puts both `gale` and `gale-lsp` on your machine in one step. `cargo install galex` still works as a fallback.',
        code: `# macOS / Linux
curl -fsSL https://get-gale.vercel.app/install.sh | sh

# Windows
irm https://get-gale.vercel.app/install.ps1 | iex

# Verify
gale --version
gale-lsp --help`,
        codeLanguage: 'bash',
      },
      {
        id: 'first-project',
        title: 'Create your first project',
        body: 'Run `gale new`, move into the directory, and start the dev server. The dev loop compiles on file save and shows a browser error overlay with GX-coded diagnostics.',
        code: `gale new my-app
cd my-app
gale dev`,
        codeLanguage: 'bash',
      },
      {
        id: 'project-shape',
        title: 'Project structure',
        body: 'Gale is file-routed. `app/page.gx` defines `/`, layouts wrap route groups, `middleware.gx` guards route segments, and `public/` is served as-is.',
        code: `my-app/
  galex.toml         # project config
  app/
    layout.gx        # root HTML shell
    page.gx          # home route /
    about/
      page.gx        # /about
    admin/
      middleware.gx  # guards /admin/**
      page.gx
  public/            # static assets
  styles/`,
        codeLanguage: 'text',
      },
      {
        id: 'first-server-flow',
        title: 'Start with a real server boundary',
        body: 'The shortest path to understanding Gale is to write a guarded action rather than a static counter. This example validates user input, sends it to a server action, and wires field-level errors back to the form.',
        code: `guard ContactForm {
  email:   string.email().trim()
  message: string.min(10).max(500)
}

server {
  out server action submitContact(form: ContactForm) -> { ok: bool } {
    db.insert('contact_messages', form)
    return { ok: true }
  }
}

out ui ContactPage {
  signal sent = false

  <form form:guard={ContactForm} form:action={submitContact}
        form:onSuccess={ _ => sent.set(true) }>
    <input name="email" type="email" />
    <textarea name="message"></textarea>
    <form:error field="email" />
    <form:error field="message" />
    <button type="submit">"Send"</button>
  </form>

  when sent.get() {
    <p>"Message sent!"</p>
  }
}`,
        codeLanguage: 'gx',
      },
      {
        id: 'build-and-ship',
        title: 'Build and ship',
        body: 'One command produces a release binary plus static assets in `dist/`. Copy the directory to any server and run the binary. No Node, no runtime dependencies.',
        code: `gale build --release
gale serve
# or run directly:
./dist/my-app --port 8080`,
        codeLanguage: 'bash',
      },
    ],
    related: ['/install', '/docs/reference', '/docs/api'],
  },
  {
    path: '/docs/reference',
    navLabel: 'Language Reference',
    group: 'Reference',
    title: 'GaleX language reference',
    description: 'Top-level declarations, template directives, boundary blocks, type system, and routing — the full GaleX syntax reference.',
    badge: 'Syntax',
    sections: [
      {
        id: 'top-level',
        title: 'Top-level declarations',
        body: 'A `.gx` file can declare UI components, layouts, guards, actions, channels, API resources, stores, enums, env declarations, middleware, and tests. Items can appear in any order.',
        code: `out ui Page { }
out layout Root { }
guard SignupForm { }
channel Notifications -> Notification
out api Users { }
out env { }
middleware Auth for /admin { }
enum Status { Draft, Published }
store CartStore { }`,
        codeLanguage: 'gx',
      },
      {
        id: 'boundary-blocks',
        title: 'Boundary blocks',
        body: 'The `server { }`, `client { }`, and `shared { }` blocks enforce the server/client separation. The compiler\'s boundary analysis catches cross-boundary access at compile time, not at runtime.',
        code: `server {
  // server-only: DB, env secrets, sessions
  let secret = env.SESSION_SECRET
}

client {
  // client-only: signals, refs, DOM
  signal count = 0
}

shared {
  // both sides: pure functions, types, enums
  enum Role { Admin, Editor, Viewer }
}`,
        codeLanguage: 'gx',
      },
      {
        id: 'components',
        title: 'Components and layouts',
        body: '`out ui Name { }` defines a renderable component. `out layout Name { }` wraps route groups and must contain a `slot` where child pages are injected. The `head { }` block sets page metadata.',
        code: `out layout Root {
  <html lang="en">
    <body>
      <nav>...</nav>
      slot
    </body>
  </html>
}

out ui HomePage {
  head {
    title: "Home"
    description: "Welcome to my app"
  }

  <h1>"Welcome"</h1>
}`,
        codeLanguage: 'gx',
      },
      {
        id: 'template-directives',
        title: 'Template directives',
        body: 'Templates use GaleX-specific control flow. `when` replaces `{condition && <el>}`, `each` replaces `map`, `bind:` is two-way data binding, and `on:` handles events with optional modifiers.',
        code: `when user != null {
  <p>"Welcome, {user.name}"</p>
} else {
  <a href="/login">"Sign in"</a>
}

each item in items.get() key={item.id} {
  <li class:active={item.active}>{item.name}</li>
} empty {
  <li>"No items yet."</li>
}

<input bind:value={name} />
<form on:submit.prevent={save}>
  <button on:click={reset}>"Reset"</button>
</form>`,
        codeLanguage: 'gx',
      },
      {
        id: 'reactivity',
        title: 'Reactivity',
        body: 'Gale uses fine-grained reactivity. `signal` is a mutable reactive value. `derive` computes from signals. `effect` runs side effects. `batch` groups multiple signal mutations into a single DOM update pass.',
        code: `signal count  = 0
signal name   = "World"

derive doubled  = count.get() * 2
derive greeting = "Hello, {name.get()}!"

effect {
  console.log("count changed:", count.get())
}

watch name {
  document.title = "Hello " + name.get()
}

batch(() => {
  count.set(10)
  name.set("Gale")
})`,
        codeLanguage: 'gx',
      },
      {
        id: 'routing',
        title: 'File-based routing',
        body: 'Routes are defined by directory structure. Dynamic segments use brackets. Catch-alls use `[...rest]`. Layouts, middleware, guards, and error boundaries all follow the same file-based convention.',
        code: `app/page.gx              -> /
app/about/page.gx        -> /about
app/blog/[slug]/page.gx  -> /blog/:slug
app/[...rest]/page.gx    -> /404 catch-all
app/admin/middleware.gx  -> guards /admin/**
app/admin/page.gx        -> /admin`,
        codeLanguage: 'text',
      },
      {
        id: 'types',
        title: 'Type system',
        body: 'GaleX uses structural typing with inference. Primitives are `int`, `float`, `string`, `bool`. Optional types use `?`. Arrays use `[]`. Generics use `<T>`. The compiler distinguishes `int` from `float` and enforces this at boundaries.',
        code: `let name:  string  = "Gale"
let count: int     = 42
let price: float   = 9.99
let items: string[] = ["a", "b"]
let tag:   string? = null

type User = {
  id:    int
  name:  string
  email: string
}

enum Status { Draft, Published, Archived }`,
        codeLanguage: 'gx',
      },
    ],
    related: ['/docs/api', '/docs/guides/forms', '/docs/getting-started'],
  },
  {
    path: '/docs/api',
    navLabel: 'API Reference',
    group: 'Reference',
    title: 'Server primitives and runtime surface',
    description: 'Guards, actions, API resources, channels, queries, and the client runtime — everything that makes Gale more than a page compiler.',
    badge: 'Server features',
    sections: [
      {
        id: 'guards',
        title: 'Guards',
        body: 'Guards are typed input contracts. Use them for form validation, action parameters, env variables, API payloads, and channel messages. The compiler emits GX-coded errors when guard usage is incorrect.',
        table: {
          headers: ['Validator', 'Types', 'Description'],
          rows: [
            ['.min(n)', 'string, int, float', 'Minimum length or value'],
            ['.max(n)', 'string, int, float', 'Maximum length or value'],
            ['.email()', 'string', 'Validates email format'],
            ['.url()', 'string', 'Validates URL format'],
            ['.uuid()', 'string', 'Validates UUID format'],
            ['.regex(pattern)', 'string', 'Must match the regex'],
            ['.trim()', 'string', 'Strip whitespace before validation'],
            ['.positive()', 'int, float', 'Must be greater than zero'],
            ['.optional()', 'any', 'Field may be absent'],
            ['.default(value)', 'any', 'Default when field is absent'],
          ],
        },
      },
      {
        id: 'actions',
        title: 'Server actions',
        body: 'Server actions are the mutation surface from UI to server-only capabilities. They are typed, serializable, and boundary-checked at compile time. Calling an action from the client triggers an RPC over HTTPS.',
        code: `server {
  out server action createPost(input: CreatePost) -> Post {
    let post = db.insert('posts', input)
    mailer.notify(post.authorEmail, "Post published")
    return post
  }

  out server action deletePost(id: int) -> { ok: bool } {
    db.delete('posts', { id })
    return { ok: true }
  }
}`,
        codeLanguage: 'gx',
      },
      {
        id: 'api-routes',
        title: 'API resources',
        body: 'Use `out api` for a stable HTTP surface for external callers, webhooks, or programmatic clients. Use actions when the caller is your own app\'s UI. Both live in `app/` using file-based routing.',
        code: `// app/api/orders/page.gx
out api Orders {
  get() -> Order[] {
    return db.query('select * from orders')
  }

  post(input: CreateOrder) -> Order {
    return db.insert('orders', input)
  }
}

// app/api/webhooks/page.gx
out api Webhooks {
  post() -> { received: bool } {
    payments.handleWebhook(request.body)
    return { received: true }
  }
}`,
        codeLanguage: 'gx',
      },
      {
        id: 'queries',
        title: 'Queries',
        body: 'Queries are client-side reactive data fetching with caching, background revalidation, and retries. They return an object with `.data`, `.loading`, `.error`, `.refetch()`, and `.mutate()` signals.',
        code: `client {
  // Basic query
  signal users = query('/api/users')

  // With options
  signal orders = query('/api/orders', {
    staleTime: 30_000,
    retries: 3,
  })

  // Reactive URL (refetches when signal changes)
  signal userId = 1
  signal user   = query('/api/users/{userId.get()}')
}`,
        codeLanguage: 'gx',
      },
      {
        id: 'channels',
        title: 'Channels',
        body: 'Channels provide typed WebSocket communication. Declare them on the server with a direction (`->` server-to-client, `<->` bidirectional). Subscribe from the client with `channel()`. Auto-reconnect with exponential backoff is built in.',
        code: `shared {
  type ChatMessage = { author: string; body: string }
}

server {
  channel ChatRoom <-> ChatMessage
  channel Alerts   -> Alert           // server-to-client only
}

client {
  signal chat   = channel('ChatRoom')
  signal alerts = channel('Alerts')

  // chat.connected  — boolean signal
  // chat.messages   — signal with all received messages
  // chat.send(msg)  — send a message to the server
  // chat.close()    — disconnect
}`,
        codeLanguage: 'gx',
      },
      {
        id: 'cli',
        title: 'CLI reference',
        body: 'The `gale` CLI is the single entry point for development, building, checking, linting, formatting, and testing.',
        table: {
          headers: ['Command', 'Description'],
          rows: [
            ['gale new [name]', 'Create a new project'],
            ['gale dev', 'Start dev server with hot reload and error overlay'],
            ['gale build [--release]', 'Compile to Rust binary'],
            ['gale serve', 'Run the production build from dist/'],
            ['gale check', 'Type-check without building'],
            ['gale fmt [--check]', 'Format .gx files'],
            ['gale lint', 'Run static analysis'],
            ['gale test [--filter]', 'Run test blocks'],
            ['gale build --docker', 'Produce a Dockerfile alongside the release build'],
          ],
        },
      },
    ],
    related: ['/docs/guides/forms', '/docs/guides/database', '/docs/guides/realtime'],
  },
  {
    path: '/docs/config',
    navLabel: 'Config',
    group: 'Reference',
    title: 'Gale.toml configuration reference',
    description: 'Every option in Gale.toml, with secure defaults and environment variable overrides documented.',
    badge: 'Config',
    sections: [
      {
        id: 'overview',
        title: 'File-optional, env-overridable',
        body: '`Gale.toml` is optional — Gale runs securely with all defaults. Environment variables (`GALE_PORT`, `GALE_ROOT`, etc.) override file values so the same binary moves through staging and production without rebuilds.',
      },
      {
        id: 'server',
        title: 'Server',
        body: 'Core server binding, document root, index file, health endpoint, and graceful shutdown timeout.',
        code: `[server]
bind                 = "0.0.0.0"
port                 = 8080
root                 = "./public"
index                = "index.html"
error_page_404       = ""           # path to custom 404.html
health_endpoint      = "/health"
shutdown_timeout_secs = 10`,
        codeLanguage: 'toml',
      },
      {
        id: 'tls',
        title: 'TLS',
        body: 'Static certificate mode and automatic ACME (Let\'s Encrypt) are both supported. ACME uses the staging environment until `acme_production = true`.',
        code: `[tls]
enabled       = false
cert          = ""               # /path/to/fullchain.pem
key           = ""               # /path/to/privkey.pem
acme          = false            # auto Let's Encrypt
acme_email    = ""
acme_domain   = ""
acme_cache_dir = "./acme_cache"
acme_production = false`,
        codeLanguage: 'toml',
      },
      {
        id: 'security',
        title: 'Security',
        body: 'All security headers are on by default. Dotfile blocking applies on all platforms. On Windows, files with the hidden attribute are also blocked.',
        code: `[security]
csp                    = "default-src 'self'"
hsts_max_age           = 31536000
hsts_include_subdomains = true
x_content_type_options = true
x_frame_options        = "DENY"
referrer_policy        = "strict-origin-when-cross-origin"
permissions_policy     = "camera=(), microphone=(), geolocation=()"
server_header          = ""      # empty = don't send
block_dotfiles         = true`,
        codeLanguage: 'toml',
      },
      {
        id: 'compression',
        title: 'Compression',
        body: 'Brotli and gzip are enabled by default. Pre-compressed `.br` / `.gz` sidecar files are served directly. Already-compressed formats (images, video, fonts, archives) are skipped.',
        code: `[compression]
enabled        = true
min_size       = 1024      # bytes — skip tiny files
algorithms     = ["br", "gzip"]
pre_compressed = true`,
        codeLanguage: 'toml',
      },
      {
        id: 'rate-limit',
        title: 'Rate limiting',
        body: 'Per-IP token bucket rate limiting. Adjust burst and per-second values for your traffic profile.',
        code: `[rate_limit]
enabled              = true
requests_per_second  = 100
max_connections_per_ip = 256
burst                = 50`,
        codeLanguage: 'toml',
      },
      {
        id: 'cache',
        title: 'Caching',
        body: 'HTML is never cached. Fingerprinted assets get a one-year immutable cache header. Override the lists per extension.',
        code: `[cache]
default_max_age    = 3600        # 1 hour for HTML
immutable_max_age  = 31536000    # 1 year for hashed assets
no_cache_extensions = ["html", "htm"]`,
        codeLanguage: 'toml',
      },
    ],
    related: ['/install', '/docs/guides/deploying'],
  },
  {
    path: '/docs/guides/forms',
    navLabel: 'Forms',
    group: 'Guides',
    title: 'Forms and validation',
    description: 'Use guards, actions, and field-aware UI to build forms that validate once and stay typed all the way through the server.',
    badge: 'Guide',
    sections: [
      {
        id: 'the-pattern',
        title: 'The core pattern',
        body: 'Gale forms are end-to-end typed. A guard defines the shape and constraints, a server action receives validated data, and the template wires them together. Validation runs client-side and server-side from the same guard.',
        code: `guard ContactForm {
  name:    string.min(1).max(100).trim()
  email:   string.email().trim()
  message: string.min(20).max(2000)
}

server {
  out server action submitContact(form: ContactForm) -> { ok: bool } {
    db.insert('contact_messages', form)
    mailer.sendAck(form.email)
    return { ok: true }
  }
}

out ui ContactPage {
  signal sent = false

  <form form:guard={ContactForm}
        form:action={submitContact}
        form:onSuccess={ _ => sent.set(true) }>

    <label>"Name"
      <input name="name" type="text" />
      <form:error field="name" />
    </label>

    <label>"Email"
      <input name="email" type="email" />
      <form:error field="email" />
    </label>

    <label>"Message"
      <textarea name="message"></textarea>
      <form:error field="message" />
    </label>

    <button type="submit">"Send message"</button>
  </form>

  when sent.get() {
    <p class="success">"Message sent!"</p>
  }
}`,
        codeLanguage: 'gx',
      },
      {
        id: 'validation-chain',
        title: 'Validation chains',
        body: 'Guards use chainable validators. The compiler checks that validators are applied to compatible types — for example, `.email()` on a non-string field is a GX0605 compile error.',
        code: `guard SignupForm {
  username:  string.min(3).max(30).regex(/^[a-z0-9_]+$/)
  email:     string.email().trim().lower()
  password:  string.min(12).max(128)
  birthYear: int.min(1900).max(2010)
  terms:     bool
}`,
        codeLanguage: 'gx',
      },
      {
        id: 'nested-guards',
        title: 'Guard composition',
        body: 'Guards can extend other guards with `&`. Use `.partial()` to make all fields optional (useful for update forms), `.pick()` to select fields, and `.omit()` to exclude them.',
        code: `guard BaseUser {
  name:  string.min(1)
  email: string.email()
}

guard CreateUser = BaseUser & {
  password: string.min(12)
}

guard UpdateUser = BaseUser.partial()`,
        codeLanguage: 'gx',
      },
    ],
    related: ['/docs/api', '/docs/guides/auth', '/examples'],
  },
  {
    path: '/docs/guides/auth',
    navLabel: 'Authentication',
    group: 'Guides',
    title: 'Authentication and protected routes',
    description: 'Combine middleware, actions, sessions, and env to keep auth server-side and properly boundary-checked.',
    badge: 'Guide',
    sections: [
      {
        id: 'full-flow',
        title: 'The full auth flow',
        body: 'Gale auth has three parts: a guarded login action creates a session, a middleware guard protects routes, and the env declaration keeps secrets server-only.',
        code: `out env {
  server {
    SESSION_SECRET: string.min(32)
    JWT_EXPIRY:     int.default(3600)
  }
}

guard LoginForm {
  email:    string.email().trim()
  password: string.min(8)
}

server {
  out server action login(form: LoginForm) -> { ok: bool; role: string } {
    let user = auth.verify(form.email, form.password)
    when user == null {
      throw { code: "INVALID_CREDENTIALS" }
    }
    session.start({ userId: user.id, role: user.role })
    return { ok: true, role: user.role }
  }

  out server action logout() {
    session.end()
    redirect('/login')
  }
}

middleware AdminOnly for /admin {
  fn handle(req, next) -> Response {
    let sess = session.get()
    when sess == null || sess.role != "admin" {
      return redirect('/login?next=' + req.path)
    }
    return next(req)
  }
}`,
        codeLanguage: 'gx',
      },
      {
        id: 'login-page',
        title: 'The login page',
        body: 'Build the login UI as a standard guarded form. Call the login action on submission and navigate on success.',
        code: `out ui LoginPage {
  head { title: "Sign in" }

  signal error = null

  <form form:guard={LoginForm}
        form:action={login}
        form:onSuccess={ result => navigate('/admin') }
        form:onError={ err => error.set(err.message) }>

    <input name="email" type="email" placeholder="Email" />
    <input name="password" type="password" placeholder="Password" />

    when error.get() != null {
      <p class="error">{error.get()}</p>
    }

    <button type="submit">"Sign in"</button>
  </form>
}`,
        codeLanguage: 'gx',
      },
    ],
    related: ['/docs/guides/database', '/docs/guides/forms'],
  },
  {
    path: '/docs/guides/database',
    navLabel: 'Database',
    group: 'Guides',
    title: 'Database patterns',
    description: 'Keep database access server-side and typed through actions, API routes, and typed env declarations.',
    badge: 'Guide',
    sections: [
      {
        id: 'config',
        title: 'Configuration',
        body: 'Set the adapter in `galex.toml` and provide the connection string at runtime through a typed env variable. This keeps secrets out of source control.',
        code: `# galex.toml
[database]
adapter = "postgres"  # or "sqlite"`,
        codeLanguage: 'toml',
      },
      {
        id: 'env',
        title: 'Typed env for connection strings',
        body: 'Declare the database URL in the server env block so the compiler enforces that it cannot cross into client code.',
        code: `out env {
  server {
    DATABASE_URL:  string.url()
    DB_POOL_SIZE:  int.min(1).max(64).default(10)
  }
}`,
        codeLanguage: 'gx',
      },
      {
        id: 'actions',
        title: 'Actions as the data layer',
        body: 'All database mutations go through server actions. This keeps business logic on the server and gives every mutation a typed interface.',
        code: `server {
  out server action createPost(input: CreatePost) -> Post {
    return db.insert('posts', {
      ...input,
      slug:      slugify(input.title),
      createdAt: now(),
      authorId:  session.get().userId,
    })
  }

  out server action loadDashboard() -> DashboardData {
    return {
      totalUsers:  db.count('users'),
      totalOrders: db.count('orders'),
      recentOrders: db.query(
        'select * from orders order by created_at desc limit 10'
      ),
    }
  }
}`,
        codeLanguage: 'gx',
      },
      {
        id: 'api-routes',
        title: 'API resources for external access',
        body: 'Use `out api` when you need a stable HTTP surface for third-party callers or webhooks rather than Gale\'s own RPC layer.',
        code: `out api Posts {
  get() -> Post[] {
    return db.query('select * from posts where published = true')
  }

  get [id: int]() -> Post {
    let post = db.find('posts', { id })
    when post == null { throw { code: "NOT_FOUND", status: 404 } }
    return post
  }

  post(input: CreatePost) -> Post {
    return db.insert('posts', input)
  }
}`,
        codeLanguage: 'gx',
      },
    ],
    related: ['/docs/api', '/docs/config', '/docs/guides/auth'],
  },
  {
    path: '/docs/guides/realtime',
    navLabel: 'Realtime',
    group: 'Guides',
    title: 'Realtime with channels',
    description: 'Build realtime features with typed WebSocket channels, presence, reconnect, and server history.',
    badge: 'Guide',
    sections: [
      {
        id: 'channel-declaration',
        title: 'Declare a channel',
        body: 'Channels are declared in the server block with a direction and message type. `->` is server-to-client. `<->` is bidirectional. The message type must be shared and JSON-serializable.',
        code: `shared {
  type ChatMessage = {
    author: string
    body:   string
    sentAt: string
  }

  type PresenceUpdate = {
    userId: int
    online: bool
  }
}

server {
  channel ChatRoom    <-> ChatMessage
  channel Presence    ->  PresenceUpdate
}`,
        codeLanguage: 'gx',
      },
      {
        id: 'client-usage',
        title: 'Subscribe on the client',
        body: 'Call `channel()` on the client to subscribe. The returned object has `.connected`, `.messages`, `.send()`, `.close()`, and `.reconnect()` built in. Reconnect uses exponential backoff up to 30 seconds.',
        code: `client {
  signal chat = channel('ChatRoom')

  // chat.connected          — bool signal
  // chat.messages           — ChatMessage[] signal
  // chat.send({ ... })      — send to server
  // chat.close()            — disconnect
}`,
        codeLanguage: 'gx',
      },
      {
        id: 'history',
        title: 'Combine with an action for history',
        body: 'Channels carry live updates. Load history with an action or query, then append channel messages on top.',
        code: `server {
  out server action loadHistory() -> ChatMessage[] {
    return db.query(
      'select * from chat_messages order by sent_at asc limit 100'
    )
  }
}

client {
  signal history = await loadHistory()
  signal room    = channel('ChatRoom')

  derive allMessages = [...history.get(), ...room.messages.get()]
}`,
        codeLanguage: 'gx',
      },
    ],
    related: ['/docs/api', '/docs/guides/auth'],
  },
  {
    path: '/docs/guides/deploying',
    navLabel: 'Deploying',
    group: 'Guides',
    title: 'Deploy Gale to production',
    description: 'Release builds, env overrides, health checks, and container-friendly deployment patterns for a single-binary stack.',
    badge: 'Guide',
    sections: [
      {
        id: 'build',
        title: 'Release build',
        body: '`gale build --release` runs the full optimization pipeline: type-check, codegen, Tailwind purge, JS minification, asset hashing, Rust release compile (LTO, strip). Output is in `dist/`.',
        code: `gale build --release

# dist/
#   my-app          — the binary (or my-app.exe on Windows)
#   public/
#     _gale/
#       runtime.a1b2.js
#       styles.c3d4.css
#     favicon.ico`,
        codeLanguage: 'bash',
      },
      {
        id: 'run',
        title: 'Run in production',
        body: 'Copy `dist/` to the server and run the binary. All config can be overridden via environment variables — no rebuild needed for config changes.',
        code: `# env-override pattern
GALE_PORT=8080 GALE_ROOT=./public ./my-app

# or with Gale.toml present
gale serve`,
        codeLanguage: 'bash',
      },
      {
        id: 'docker',
        title: 'Docker',
        body: 'The generated Dockerfile uses FROM scratch for a minimal image. The final image contains only the binary and static assets.',
        code: `gale build --release --docker
docker build -t my-app dist/
docker run -p 8080:8080 \\
  -e DATABASE_URL=postgres://... \\
  my-app`,
        codeLanguage: 'bash',
      },
      {
        id: 'health',
        title: 'Health checks and ops',
        body: 'The `/health` endpoint returns 200 OK with no body. Useful for Kubernetes liveness probes, ECS target group checks, and load balancer pings. Configurable via `Gale.toml`.',
        code: `# Kubernetes liveness probe
livenessProbe:
  httpGet:
    path: /health
    port: 8080
  initialDelaySeconds: 3
  periodSeconds: 10`,
        codeLanguage: 'yaml',
      },
    ],
    related: ['/install', '/docs/config'],
  },
  {
    path: '/docs/guides/migration',
    navLabel: 'Migration',
    group: 'Guides',
    title: 'Migrating from React / Next.js',
    description: 'Map common frontend patterns into Gale\'s boundary-aware model and binary deployment story.',
    badge: 'Guide',
    sections: [
      {
        id: 'concept-map',
        title: 'Concept mapping',
        body: 'Most React patterns map directly to Gale equivalents. The biggest difference is that "server" in Gale is a compiler-enforced boundary, not a runtime convention.',
        table: {
          headers: ['React / Next.js', 'Gale equivalent'],
          rows: [
            ['pages/index.tsx', 'app/page.gx'],
            ['pages/[slug].tsx', 'app/[slug]/page.gx'],
            ['_app.tsx / layout.tsx', 'app/layout.gx'],
            ['useState', 'signal'],
            ['useMemo / useCallback', 'derive'],
            ['useEffect', 'effect'],
            ['useRef', 'ref'],
            ['getServerSideProps', 'server { } block'],
            ['API routes (pages/api/*)', 'out api Resource { }'],
            ['Server Actions', 'out server action'],
            ['middleware.ts', 'middleware.gx'],
            ['next/head', 'head { title: ... }'],
            ['next/link', '<a href="...">'],
            ['React.memo', 'Not needed (fine-grained signals)'],
            ['Virtual DOM diffing', 'Direct DOM updates via signals'],
            ['Node.js runtime', 'Native Rust binary'],
          ],
        },
      },
      {
        id: 'template-syntax',
        title: 'Template syntax differences',
        body: 'GaleX templates look like JSX but have different control flow. Conditions and loops are block-level statements, not expressions.',
        code: `// React JSX
<div className={active ? "active" : ""}>
  {items.map(item => (
    <li key={item.id}>{item.name}</li>
  ))}
</div>

// Gale GaleX
<div class:active={isActive.get()}>
  each item in items.get() key={item.id} {
    <li>{item.name}</li>
  }
</div>`,
        codeLanguage: 'text',
      },
      {
        id: 'key-differences',
        title: 'Key operational differences',
        body: 'The migration wins that matter are not syntax changes. They are moving secrets, database access, and auth into Gale\'s server primitives so the compiler can verify the boundaries.',
        code: `# Gale produces a single deployable binary
# No node_modules, no Node.js runtime, no JS build pipeline

gale build --release
# dist/my-app (~5-10 MB stripped binary)
# dist/public/ (static assets)`,
        codeLanguage: 'bash',
      },
    ],
    related: ['/docs/getting-started', '/docs/reference'],
  },
]

export const docsNavGroups: { group: string; pages: DocPage[] }[] = Array.from(
  docsPages.reduce((map, page) => {
    const existing = map.get(page.group) ?? []
    existing.push(page)
    map.set(page.group, existing)
    return map
  }, new Map<string, DocPage[]>()),
).map(([group, pages]) => ({ group, pages }))

export function getDocByPath(path: string): DocPage | undefined {
  return docsPages.find((page) => page.path === path)
}
