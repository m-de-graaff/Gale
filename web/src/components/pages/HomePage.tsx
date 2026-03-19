import { Link } from 'react-router-dom'
import { ArrowRight, Shield, Cpu, Terminal, Zap, FileCode, Eye, GitBranch, Box } from 'lucide-react'
import { Card, CardContent } from '@/components/ui/Card'
import { CodeBlock } from '@/components/ui/CodeBlock'
import { Badge } from '@/components/ui/Badge'
import { Button } from '@/components/ui/Button'
import { Tabs } from '@/components/ui/Tabs'

const HERO_CODE = `out ui ContactPage() {
  guard ContactForm {
    email: string.trim().email()
    message: string.trim().minLen(10).maxLen(500)
  }

  server {
    action submit(data: ContactForm) -> string {
      let key = env.SENDGRID_KEY
      await send_email(data.email, data.message)
      return "Message sent"
    }
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
    </form>
  </section>
}`

const BOUNDARY_CODE = `server {
  let secret = env.API_SECRET
  action charge(amount: int) -> bool {
    let key = env.STRIPE_KEY
    return process_payment(key, amount)
  }
}

client {
  signal count = 0
  let x = secret  // GX0500: Cannot access
                  // server binding 'secret'
                  // in client scope
}`

const GUARD_CODE = `guard CreateUser {
  name: string.trim().minLen(2).maxLen(50)
  email: string.trim().email()
  age: int.min(13).max(150)
  role: string.oneOf("user", "admin").default("user")
  bio: string.optional().maxLen(500)
}

// 28 chain methods, compile-time validated
// GX0605 if .email() on non-string
// GX0603 if .min() > .max()
// GX0619 if .oneOf() is empty`

const METRICS = [
  { value: '331', label: 'Error codes', detail: 'Stable GX codes across 14 subsystems' },
  { value: '16', label: 'CLI commands', detail: 'From scaffolding to publishing' },
  { value: '28', label: 'Validators', detail: 'Guard chain methods, type-checked' },
  { value: '3', label: 'Codegen targets', detail: 'Rust server + JS client + CSS' },
]

const INSTALL_TABS = [
  {
    label: 'macOS / Linux',
    content: <div className="p-4"><CodeBlock code="curl -fsSL https://get-gale.dev/install.sh | sh" language="bash" /></div>,
  },
  {
    label: 'Windows',
    content: <div className="p-4"><CodeBlock code="irm https://get-gale.dev/install.ps1 | iex" language="bash" /></div>,
  },
  {
    label: 'Cargo',
    content: <div className="p-4"><CodeBlock code="cargo install galex" language="bash" /></div>,
  },
]

/* ── SVG Illustrations ──────────────────────────────────────────────── */

function TypeGraphSvg() {
  return (
    <svg viewBox="0 0 200 120" fill="none" className="w-full h-full opacity-80">
      {/* Nodes */}
      <circle cx="40" cy="30" r="14" className="fill-accent/10 stroke-accent/40" strokeWidth="1" />
      <circle cx="100" cy="20" r="14" className="fill-accent/10 stroke-accent/40" strokeWidth="1" />
      <circle cx="160" cy="35" r="14" className="fill-accent/10 stroke-accent/40" strokeWidth="1" />
      <circle cx="60" cy="80" r="14" className="fill-accent/10 stroke-accent/40" strokeWidth="1" />
      <circle cx="130" cy="90" r="14" className="fill-accent/10 stroke-accent/40" strokeWidth="1" />
      {/* Edges */}
      <line x1="52" y1="37" x2="88" y2="23" className="stroke-accent/20" strokeWidth="1" />
      <line x1="112" y1="27" x2="148" y2="32" className="stroke-accent/20" strokeWidth="1" />
      <line x1="46" y1="43" x2="55" y2="68" className="stroke-accent/20" strokeWidth="1" />
      <line x1="106" y1="33" x2="126" y2="77" className="stroke-accent/20" strokeWidth="1" />
      <line x1="72" y1="83" x2="118" y2="88" className="stroke-accent/20" strokeWidth="1" />
      <line x1="154" y1="47" x2="136" y2="78" className="stroke-accent/20" strokeWidth="1" />
      {/* Labels */}
      <text x="40" y="34" textAnchor="middle" className="fill-accent text-[7px] font-mono">int</text>
      <text x="100" y="24" textAnchor="middle" className="fill-accent text-[7px] font-mono">Signal</text>
      <text x="160" y="39" textAnchor="middle" className="fill-accent text-[7px] font-mono">Guard</text>
      <text x="60" y="84" textAnchor="middle" className="fill-accent text-[7px] font-mono">string</text>
      <text x="130" y="94" textAnchor="middle" className="fill-accent text-[7px] font-mono">fn()</text>
    </svg>
  )
}

function BinaryOutputSvg() {
  return (
    <svg viewBox="0 0 240 100" fill="none" className="w-full h-full opacity-80">
      {/* Source files */}
      <rect x="10" y="15" width="32" height="40" rx="4" className="fill-accent/8 stroke-accent/30" strokeWidth="1" />
      <rect x="50" y="15" width="32" height="40" rx="4" className="fill-accent/8 stroke-accent/30" strokeWidth="1" />
      <rect x="90" y="15" width="32" height="40" rx="4" className="fill-accent/8 stroke-accent/30" strokeWidth="1" />
      <text x="26" y="39" textAnchor="middle" className="fill-accent/60 text-[6px] font-mono">.gx</text>
      <text x="66" y="39" textAnchor="middle" className="fill-accent/60 text-[6px] font-mono">.gx</text>
      <text x="106" y="39" textAnchor="middle" className="fill-accent/60 text-[6px] font-mono">.gx</text>
      {/* Arrow */}
      <path d="M130 35 L155 35" className="stroke-accent/30" strokeWidth="1.5" strokeLinecap="round" markerEnd="url(#arrowhead)" />
      <defs><marker id="arrowhead" markerWidth="6" markerHeight="4" refX="5" refY="2" orient="auto"><polygon points="0 0, 6 2, 0 4" className="fill-accent/30" /></marker></defs>
      {/* Binary */}
      <rect x="162" y="10" width="65" height="50" rx="6" className="fill-accent/12 stroke-accent/40" strokeWidth="1.5" />
      <text x="194" y="30" textAnchor="middle" className="fill-accent text-[7px] font-mono font-bold">gale_app</text>
      <text x="194" y="42" textAnchor="middle" className="fill-accent/40 text-[5px] font-mono">4.2 MB binary</text>
      {/* Labels */}
      <text x="66" y="72" textAnchor="middle" className="fill-muted-foreground text-[6px]">source files</text>
      <text x="194" y="76" textAnchor="middle" className="fill-muted-foreground text-[6px]">single binary</text>
      <text x="142" y="28" textAnchor="middle" className="fill-accent/40 text-[6px] font-mono">build</text>
    </svg>
  )
}

export function HomePage() {
  return (
    <div className="flex-1">
      {/* Hero */}
      <section className="relative overflow-hidden">
        <div className="absolute inset-0 opacity-[0.025]" style={{
          backgroundImage: 'linear-gradient(hsl(var(--border)) 1px, transparent 1px), linear-gradient(90deg, hsl(var(--border)) 1px, transparent 1px)',
          backgroundSize: '48px 48px',
        }} />
        <div className="absolute top-[-200px] right-[-100px] w-[600px] h-[600px] rounded-full bg-accent/[0.04] blur-[120px] animate-pulse" style={{ animationDuration: '6s' }} />

        <div className="relative max-w-6xl mx-auto px-4 sm:px-6 pt-16 pb-20 sm:pt-24 sm:pb-28">
          <div className="flex flex-col lg:flex-row gap-12 lg:gap-16 items-start">
            <div className="flex-1 max-w-xl fade-in-up">
              <Badge variant="warning" className="mb-6">Alpha &mdash; early access</Badge>
              <h1 className="text-[2.5rem] sm:text-[3.25rem] font-extrabold leading-[1.05] tracking-[-0.035em] mb-5">
                Write <span className="bg-gradient-to-r from-accent to-emerald-300 bg-clip-text text-transparent">.gx</span> files.
                <br />
                Ship one binary.
              </h1>
              <p className="text-[15px] sm:text-base text-muted-foreground leading-relaxed mb-8 max-w-md">
                GaleX is a Rust-native web language with typed server/client boundaries, guards, actions, and SSR. The compiler generates a standalone binary via Axum.
              </p>
              <div className="flex flex-wrap gap-3">
                <Link to="/docs/getting-started">
                  <Button variant="accent" size="lg">Get Started <ArrowRight className="w-3.5 h-3.5" /></Button>
                </Link>
                <Link to="/install">
                  <Button variant="outline" size="lg">Install</Button>
                </Link>
              </div>
            </div>
            <div className="flex-1 w-full lg:max-w-lg fade-in-up" style={{ animationDelay: '0.15s' }}>
              <CodeBlock code={HERO_CODE} language="gx" filename="app/contact/page.gx" showLineNumbers />
            </div>
          </div>
        </div>
      </section>

      {/* Metrics */}
      <section className="border-y border-border/40 bg-card/30">
        <div className="max-w-6xl mx-auto px-4 sm:px-6 py-8">
          <div className="grid grid-cols-2 md:grid-cols-4 gap-8">
            {METRICS.map((m, i) => (
              <div key={m.label} className="fade-in-up" style={{ animationDelay: `${i * 0.08}s` }}>
                <div className="text-3xl font-extrabold text-foreground tracking-tight">{m.value}</div>
                <div className="text-[12px] font-semibold text-accent mt-1">{m.label}</div>
                <div className="text-[11px] text-muted-foreground/50 mt-0.5">{m.detail}</div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Bento Features Grid */}
      <section className="max-w-6xl mx-auto px-4 sm:px-6 py-20">
        <div className="text-center mb-12 fade-in-up">
          <h2 className="text-2xl font-bold tracking-tight mb-3">What's inside</h2>
          <p className="text-[14px] text-muted-foreground max-w-lg mx-auto">
            A compiler toolchain, type system, code generator, dev server, LSP, and CLI &mdash; built from scratch in Rust.
          </p>
        </div>

        <div className="grid grid-cols-6 gap-3">
          {/* Card 1: Boundaries — wide with code preview */}
          <Card className="col-span-full lg:col-span-4 overflow-hidden group hover:border-accent/30 transition-all">
            <CardContent className="grid sm:grid-cols-2 gap-4 pt-6">
              <div className="flex flex-col justify-between">
                <div>
                  <div className="flex items-center gap-2 mb-3">
                    <div className="flex items-center justify-center w-9 h-9 rounded-full border border-border/60 bg-accent/5">
                      <Shield className="w-4 h-4 text-accent" strokeWidth={1.5} />
                    </div>
                    <Badge variant="accent">GX0500</Badge>
                  </div>
                  <h3 className="text-lg font-semibold mb-2">Compiler-enforced boundaries</h3>
                  <p className="text-[13px] text-muted-foreground leading-relaxed">
                    Server secrets physically cannot reach the client. 24 error codes (GX0500&ndash;GX0523) track every binding across server, client, and shared scopes.
                  </p>
                </div>
              </div>
              <div className="rounded-lg overflow-hidden border border-border/30">
                <CodeBlock code={BOUNDARY_CODE} language="gx" />
              </div>
            </CardContent>
          </Card>

          {/* Card 2: Type System — with SVG graph */}
          <Card className="col-span-full sm:col-span-3 lg:col-span-2 overflow-hidden group hover:border-accent/30 transition-all">
            <CardContent className="pt-6">
              <div className="h-28 mb-4 flex items-center justify-center">
                <TypeGraphSvg />
              </div>
              <h3 className="text-lg font-semibold mb-1.5">Full type inference</h3>
              <p className="text-[13px] text-muted-foreground leading-relaxed">
                Constraint-based type system with Robinson unification. Signals, guards, DOM events, and templates — all checked at compile time.
              </p>
            </CardContent>
          </Card>

          {/* Card 3: Guards — with code preview */}
          <Card className="col-span-full sm:col-span-3 lg:col-span-3 overflow-hidden group hover:border-accent/30 transition-all">
            <CardContent className="pt-6">
              <div className="flex items-center gap-2 mb-3">
                <div className="flex items-center justify-center w-9 h-9 rounded-full border border-border/60 bg-accent/5">
                  <FileCode className="w-4 h-4 text-accent" strokeWidth={1.5} />
                </div>
              </div>
              <h3 className="text-lg font-semibold mb-2">28 guard validators</h3>
              <p className="text-[13px] text-muted-foreground leading-relaxed mb-3">
                Typed validation schemas that generate both server-side Rust and client-side JavaScript. Composition via <code className="text-accent/80 bg-accent/10 px-1 rounded text-[12px]">.partial()</code>, <code className="text-accent/80 bg-accent/10 px-1 rounded text-[12px]">.pick()</code>, <code className="text-accent/80 bg-accent/10 px-1 rounded text-[12px]">.omit()</code>.
              </p>
              <div className="rounded-lg overflow-hidden border border-border/30 text-[12px]">
                <CodeBlock code={GUARD_CODE} language="gx" />
              </div>
            </CardContent>
          </Card>

          {/* Card 4: Binary Output — with SVG */}
          <Card className="col-span-full sm:col-span-3 lg:col-span-3 overflow-hidden group hover:border-accent/30 transition-all">
            <CardContent className="pt-6">
              <div className="h-24 mb-4">
                <BinaryOutputSvg />
              </div>
              <h3 className="text-lg font-semibold mb-1.5">Single binary output</h3>
              <p className="text-[13px] text-muted-foreground leading-relaxed">
                <code className="text-accent/80 bg-accent/10 px-1 rounded text-[12px]">gale build</code> compiles .gx files into a standalone Rust binary via Axum and Tokio. No Node.js, no runtime dependencies.
              </p>
            </CardContent>
          </Card>

          {/* Small feature cards */}
          {[
            { icon: Zap, title: 'SSR by default', desc: 'Pages render to HTML on the server. Interactive elements hydrate selectively. Static pages ship zero JS.' },
            { icon: Terminal, title: '16 CLI commands', desc: 'new, build, dev, check, lint, serve, add, publish, self-update, and more. One toolchain.' },
            { icon: Eye, title: 'LSP with 10 features', desc: 'Diagnostics, hover, go-to-def, references, rename, code actions, symbols, folding. VS Code + Zed.' },
            { icon: GitBranch, title: 'File-based routing', desc: 'app/page.gx becomes /. Dynamic [slug] segments. Catch-all [...rest]. Layouts nest automatically.' },
            { icon: Cpu, title: 'Dev server + HMR', desc: 'Reverse proxy with WebSocket hot reload. 50ms debounce. Error overlay. Incremental rebuilds.' },
            { icon: Box, title: 'Package registry', desc: 'gale add, remove, update, search, publish. SHA-256 verified. Lockfile managed.' },
          ].map(f => (
            <Card key={f.title} className="col-span-3 sm:col-span-3 lg:col-span-1 overflow-hidden group hover:border-accent/30 transition-all">
              <CardContent className="pt-6">
                <f.icon className="w-5 h-5 text-accent mb-3 group-hover:scale-110 transition-transform" strokeWidth={1.5} />
                <h3 className="text-[14px] font-semibold mb-1">{f.title}</h3>
                <p className="text-[11px] text-muted-foreground/70 leading-relaxed">{f.desc}</p>
              </CardContent>
            </Card>
          ))}
        </div>
      </section>

      {/* Install */}
      <section className="border-t border-border/40 bg-card/20">
        <div className="max-w-6xl mx-auto px-4 sm:px-6 py-20">
          <div className="grid lg:grid-cols-2 gap-10 items-start">
            <div>
              <h2 className="text-2xl font-bold tracking-tight mb-3">One command install</h2>
              <p className="text-[14px] text-muted-foreground leading-relaxed mb-5">
                The SDK installer puts <code className="text-accent/80 bg-accent/10 px-1 py-0.5 rounded text-[12px]">gale</code> (CLI) and <code className="text-accent/80 bg-accent/10 px-1 py-0.5 rounded text-[12px]">gale-lsp</code> (language server) on your PATH. No admin rights required.
              </p>
              <ul className="space-y-2 text-[13px] text-muted-foreground">
                <li className="flex items-start gap-2"><span className="text-accent mt-0.5">&#x2022;</span><span><strong className="text-foreground">gale</strong> &mdash; new, dev, build, check, lint, serve, add, publish</span></li>
                <li className="flex items-start gap-2"><span className="text-accent mt-0.5">&#x2022;</span><span><strong className="text-foreground">gale-lsp</strong> &mdash; diagnostics, hover, go-to-def, rename, references</span></li>
              </ul>
            </div>
            <Tabs tabs={INSTALL_TABS} />
          </div>
        </div>
      </section>

      {/* Docs grid */}
      <section className="max-w-6xl mx-auto px-4 sm:px-6 py-20">
        <div className="text-center mb-12">
          <h2 className="text-2xl font-bold tracking-tight mb-3">Learn more</h2>
          <p className="text-[14px] text-muted-foreground">Guides, references, and patterns for building with GaleX.</p>
        </div>
        <div className="grid sm:grid-cols-2 lg:grid-cols-3 gap-3">
          {[
            { title: 'Getting Started', desc: 'Install, scaffold a project, run the dev server.', href: '/docs/getting-started' },
            { title: 'Guards', desc: '28 validators, composition, compile-time type checks.', href: '/docs/reference/guards' },
            { title: 'Boundaries', desc: 'server/client/shared blocks, 24 error codes.', href: '/docs/reference/boundaries' },
            { title: 'Templates', desc: 'Directives, conditionals, lists, DOM type checking.', href: '/docs/reference/templates' },
            { title: 'CLI Commands', desc: '16 commands: new, build, dev, check, lint, publish.', href: '/docs/cli/project' },
            { title: 'Deploying', desc: 'Release builds, Docker, health checks, systemd.', href: '/docs/guides/deploying' },
          ].map(card => (
            <Link key={card.href} to={card.href}>
              <Card className="h-full hover:border-accent/30 transition-all group">
                <CardContent className="pt-5 pb-5">
                  <h3 className="text-[14px] font-semibold mb-1.5 group-hover:text-accent transition-colors">{card.title}</h3>
                  <p className="text-[12px] text-muted-foreground/70 leading-relaxed">{card.desc}</p>
                  <span className="inline-flex items-center gap-1 mt-3 text-[12px] text-accent/70 group-hover:text-accent transition-colors">
                    Read more <ArrowRight className="w-3 h-3" />
                  </span>
                </CardContent>
              </Card>
            </Link>
          ))}
        </div>
      </section>
    </div>
  )
}
