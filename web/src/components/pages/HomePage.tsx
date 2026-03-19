import { Link } from 'react-router-dom'
import { ArrowRight, Shield, Cpu, Terminal, Zap, FileCode, Eye, GitBranch, Box } from 'lucide-react'
import { CodeBlock } from '@/components/ui/CodeBlock'
import { Badge } from '@/components/ui/Badge'
import { Tabs } from '@/components/ui/Tabs'

const HERO_CODE = `out ui ContactPage() {
  server {
    action submit(data: ContactForm) -> string {
      // Runs on the server. Cannot leak to the client.
      let api_key = env.SENDGRID_KEY
      await send_email(data.email, data.message)
      return "Message sent"
    }
  }

  guard ContactForm {
    email: string.trim().email()
    message: string.trim().minLen(10).maxLen(500)
  }

  client {
    signal status = ""
  }

  <section>
    <h1>Contact us</h1>
    <form form:action={submit} form:guard={ContactForm}>
      <input bind:value={email} type="email" />
      <textarea bind:value={message} />
      <button type="submit">Send</button>
      <when status != "">
        <p>{status}</p>
      </when>
    </form>
  </section>
}`

const BOUNDARY_CODE = `server {
  let secret = env.API_SECRET   // Server-only
  action charge(amount: int) -> bool {
    let key = env.STRIPE_KEY    // Never leaves server
    return process_payment(key, amount)
  }
}

client {
  signal count = 0
  let x = secret    // GX0500: Cannot access
                    // server binding 'secret'
                    // in client scope
}`

const FEATURES = [
  {
    icon: Shield,
    title: 'Compiler-enforced boundaries',
    description: 'Server secrets physically cannot reach the client. The compiler tracks every binding across server, client, and shared scopes and rejects cross-boundary access at compile time.',
  },
  {
    icon: Cpu,
    title: 'Full type inference',
    description: 'Constraint-based type system with Robinson unification. Signal types, guard validator compatibility, DOM event types, and template directives are all checked before a single line of Rust is generated.',
  },
  {
    icon: FileCode,
    title: 'Single-file components',
    description: 'Each .gx file declares its guards, actions, signals, and template in one place. No separate API routes, no schema files, no build configuration.',
  },
  {
    icon: Zap,
    title: 'SSR by default',
    description: 'Pages render to HTML on the server. Interactive elements hydrate selectively on the client. Static pages ship zero JavaScript.',
  },
  {
    icon: Terminal,
    title: '18 CLI commands',
    description: 'new, build, dev, check, lint, serve, add, remove, update, search, publish, login, self-update, editor install, and more. One toolchain, no plugins.',
  },
  {
    icon: Eye,
    title: 'LSP with 10 features',
    description: 'Diagnostics, hover types, go-to-definition, find references, rename, code actions, document symbols, and folding. Works in VS Code and Zed.',
  },
  {
    icon: GitBranch,
    title: 'File-based routing',
    description: 'app/page.gx becomes /. app/about/page.gx becomes /about. Dynamic segments with [slug] and catch-alls with [...rest]. Layouts nest automatically.',
  },
  {
    icon: Box,
    title: 'Single binary output',
    description: 'gale build compiles your .gx files into a standalone Rust binary via Axum and Tokio. No Node.js, no runtime dependencies. Deploy one file.',
  },
]

const METRICS = [
  { label: 'Error codes', value: '331', detail: 'Stable, documented GX codes across 14 subsystems' },
  { label: 'CLI commands', value: '18', detail: 'From scaffolding to publishing' },
  { label: 'Checker modules', value: '15', detail: 'Boundary, guard, store, reactivity, template, DOM, and more' },
  { label: 'Codegen targets', value: '3', detail: 'Rust server + JS client + CSS' },
]

const INSTALL_TABS = [
  {
    label: 'macOS / Linux',
    content: (
      <div className="p-4">
        <CodeBlock code="curl -fsSL https://get-gale.dev/install.sh | sh" language="bash" />
      </div>
    ),
  },
  {
    label: 'Windows',
    content: (
      <div className="p-4">
        <CodeBlock code="irm https://get-gale.dev/install.ps1 | iex" language="bash" />
      </div>
    ),
  },
  {
    label: 'Cargo',
    content: (
      <div className="p-4">
        <CodeBlock code="cargo install galex" language="bash" />
      </div>
    ),
  },
]

export function HomePage() {
  return (
    <div className="flex-1">
      {/* Hero */}
      <section className="relative overflow-hidden">
        {/* Background grid */}
        <div className="absolute inset-0 opacity-[0.03]" style={{
          backgroundImage: 'linear-gradient(hsl(var(--border)) 1px, transparent 1px), linear-gradient(90deg, hsl(var(--border)) 1px, transparent 1px)',
          backgroundSize: '48px 48px',
        }} />
        {/* Gradient orb */}
        <div className="absolute top-[-200px] right-[-100px] w-[600px] h-[600px] rounded-full bg-accent/[0.04] blur-[120px]" />

        <div className="relative max-w-6xl mx-auto px-4 sm:px-6 pt-16 pb-20 sm:pt-24 sm:pb-28">
          <div className="flex flex-col lg:flex-row gap-12 lg:gap-16 items-start">
            {/* Left: text */}
            <div className="flex-1 max-w-xl">
              <Badge variant="warning" className="mb-6">Alpha &mdash; early access</Badge>
              <h1 className="text-[2.5rem] sm:text-[3.25rem] font-extrabold leading-[1.05] tracking-[-0.035em] mb-5">
                Write <span className="text-accent">.gx</span> files.
                <br />
                Ship one binary.
              </h1>
              <p className="text-[15px] sm:text-base text-muted-foreground leading-relaxed mb-8 max-w-md">
                Gale is a Rust-native web framework. GaleX is the language &mdash; <code className="text-accent/80 bg-accent/10 px-1.5 py-0.5 rounded text-[13px]">.gx</code> files with typed server/client boundaries, guards, actions, and SSR. The compiler generates a standalone Rust binary via Axum.
              </p>
              <div className="flex flex-wrap gap-3">
                <Link
                  to="/docs/getting-started"
                  className="inline-flex items-center gap-2 px-5 py-2.5 rounded-lg bg-accent text-accent-foreground text-[13px] font-semibold hover:bg-accent/90 transition-colors"
                >
                  Get Started
                  <ArrowRight className="w-3.5 h-3.5" />
                </Link>
                <Link
                  to="/install"
                  className="inline-flex items-center gap-2 px-5 py-2.5 rounded-lg border border-border/60 text-[13px] font-medium text-foreground hover:bg-muted/50 transition-colors"
                >
                  Install
                </Link>
              </div>
            </div>

            {/* Right: code */}
            <div className="flex-1 w-full lg:max-w-lg">
              <CodeBlock
                code={HERO_CODE}
                language="gx"
                filename="app/contact/page.gx"
                showLineNumbers
              />
            </div>
          </div>
        </div>
      </section>

      {/* Metrics strip */}
      <section className="border-y border-border/40 bg-card/30">
        <div className="max-w-6xl mx-auto px-4 sm:px-6 py-6">
          <div className="grid grid-cols-2 md:grid-cols-4 gap-6">
            {METRICS.map(m => (
              <div key={m.label}>
                <div className="text-2xl font-bold text-foreground tracking-tight">{m.value}</div>
                <div className="text-[12px] font-medium text-muted-foreground mt-0.5">{m.label}</div>
                <div className="text-[11px] text-muted-foreground/50 mt-0.5">{m.detail}</div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Boundary enforcement visual */}
      <section className="max-w-6xl mx-auto px-4 sm:px-6 py-20">
        <div className="grid lg:grid-cols-2 gap-10 items-start">
          <div>
            <h2 className="text-2xl font-bold tracking-tight mb-3">
              Compiler-enforced boundaries
            </h2>
            <p className="text-[14px] text-muted-foreground leading-relaxed mb-4">
              Server code and client code live in the same <code className="text-accent/80 bg-accent/10 px-1 py-0.5 rounded text-[12px]">.gx</code> file but in separate boundary blocks. The type checker tracks every binding's scope and rejects cross-boundary access at compile time with stable error codes.
            </p>
            <div className="flex flex-wrap gap-2 mb-4">
              <Badge variant="accent">GX0500</Badge>
              <span className="text-[12px] text-muted-foreground">Server binding accessed from client scope</span>
            </div>
            <p className="text-[13px] text-muted-foreground/70">
              24 boundary error codes (GX0500&ndash;GX0523) cover cross-scope access, serializability of boundary-crossing types, export coherence, and env variable visibility.
            </p>
          </div>
          <CodeBlock code={BOUNDARY_CODE} language="gx" filename="boundary.gx" showLineNumbers />
        </div>
      </section>

      {/* Features grid */}
      <section className="border-t border-border/40 bg-card/20">
        <div className="max-w-6xl mx-auto px-4 sm:px-6 py-20">
          <div className="text-center mb-12">
            <h2 className="text-2xl font-bold tracking-tight mb-3">What's inside</h2>
            <p className="text-[14px] text-muted-foreground max-w-lg mx-auto">
              A compiler toolchain, type system, code generator, dev server, LSP, and CLI &mdash; built from scratch in Rust.
            </p>
          </div>
          <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-4">
            {FEATURES.map(f => (
              <div
                key={f.title}
                className="p-5 rounded-lg border border-border/40 bg-card/40 hover:border-border/80 hover:bg-card/70 transition-all group"
              >
                <f.icon className="w-5 h-5 text-accent mb-3 group-hover:scale-110 transition-transform" />
                <h3 className="text-[14px] font-semibold mb-1.5">{f.title}</h3>
                <p className="text-[12px] text-muted-foreground/70 leading-relaxed">{f.description}</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Install */}
      <section className="max-w-6xl mx-auto px-4 sm:px-6 py-20">
        <div className="grid lg:grid-cols-2 gap-10 items-start">
          <div>
            <h2 className="text-2xl font-bold tracking-tight mb-3">
              One command install
            </h2>
            <p className="text-[14px] text-muted-foreground leading-relaxed mb-5">
              The SDK installer puts <code className="text-accent/80 bg-accent/10 px-1 py-0.5 rounded text-[12px]">gale</code> (CLI) and <code className="text-accent/80 bg-accent/10 px-1 py-0.5 rounded text-[12px]">gale-lsp</code> (language server) on your PATH. No admin rights required.
            </p>
            <ul className="space-y-2 text-[13px] text-muted-foreground">
              <li className="flex items-start gap-2">
                <span className="text-accent mt-0.5">&#x2022;</span>
                <span><strong className="text-foreground">gale</strong> &mdash; new, dev, build, check, lint, serve, add, publish, and more</span>
              </li>
              <li className="flex items-start gap-2">
                <span className="text-accent mt-0.5">&#x2022;</span>
                <span><strong className="text-foreground">gale-lsp</strong> &mdash; diagnostics, hover, go-to-def, rename, references</span>
              </li>
              <li className="flex items-start gap-2">
                <span className="text-accent mt-0.5">&#x2022;</span>
                <span>Installs to <code className="text-[12px] bg-muted px-1 rounded">~/.local/bin</code> (Unix) or <code className="text-[12px] bg-muted px-1 rounded">%LOCALAPPDATA%\Gale\bin</code> (Windows)</span>
              </li>
            </ul>
          </div>
          <Tabs tabs={INSTALL_TABS} />
        </div>
      </section>

      {/* Docs grid */}
      <section className="border-t border-border/40 bg-card/20">
        <div className="max-w-6xl mx-auto px-4 sm:px-6 py-20">
          <div className="text-center mb-12">
            <h2 className="text-2xl font-bold tracking-tight mb-3">Learn more</h2>
            <p className="text-[14px] text-muted-foreground">
              Guides, references, and patterns for building with GaleX.
            </p>
          </div>
          <div className="grid sm:grid-cols-2 lg:grid-cols-3 gap-4">
            {[
              { title: 'Getting Started', desc: 'Install, create a project, and run the dev server.', href: '/docs/getting-started' },
              { title: 'Language Reference', desc: 'Every .gx construct: guards, actions, channels, stores, templates.', href: '/docs/reference' },
              { title: 'API Reference', desc: 'Guards, actions, queries, channels, stores, middleware, and env.', href: '/docs/api' },
              { title: 'CLI Reference', desc: 'All 18 commands with usage, flags, and status.', href: '/docs/cli' },
              { title: 'Config Reference', desc: 'Gale.toml and galex.toml options for server and compiler.', href: '/docs/config' },
              { title: 'Deploying', desc: 'Release builds, Docker, health checks, and env overrides.', href: '/docs/guides/deploying' },
            ].map(card => (
              <Link
                key={card.href}
                to={card.href}
                className="p-5 rounded-lg border border-border/40 bg-card/40 hover:border-accent/30 hover:bg-card/70 transition-all group"
              >
                <h3 className="text-[14px] font-semibold mb-1.5 group-hover:text-accent transition-colors">{card.title}</h3>
                <p className="text-[12px] text-muted-foreground/70 leading-relaxed">{card.desc}</p>
                <span className="inline-flex items-center gap-1 mt-3 text-[12px] text-accent/70 group-hover:text-accent transition-colors">
                  Read more <ArrowRight className="w-3 h-3" />
                </span>
              </Link>
            ))}
          </div>
        </div>
      </section>
    </div>
  )
}
