import { Link } from 'react-router-dom'
import { ArrowRight, Shield, Cpu, Terminal, Zap, FileCode, Eye, GitBranch, Box, Binary, Lock, Layers } from 'lucide-react'
import { motion } from 'framer-motion'
import { Card, CardContent } from '@/components/ui/Card'
import { CodeBlock } from '@/components/ui/CodeBlock'
import { Badge } from '@/components/ui/Badge'
import { Button } from '@/components/ui/Button'
import { Tabs } from '@/components/ui/Tabs'

/* ── Data ────────────────────────────────────────────────────────────── */

const TITLE_WORDS = ['WRITE', '.GX', 'FILES,', 'SHIP', 'ONE', 'BINARY']

const HERO_LABELS = [
  { icon: Lock, label: 'Typed Boundaries' },
  { icon: Layers, label: 'SSR by Default' },
  { icon: Binary, label: 'Single Binary' },
]

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
  { label: 'macOS / Linux', content: <div className="p-4"><CodeBlock code="curl -fsSL https://get-gale.dev/install.sh | sh" language="bash" /></div> },
  { label: 'Windows', content: <div className="p-4"><CodeBlock code="irm https://get-gale.dev/install.ps1 | iex" language="bash" /></div> },
  { label: 'Cargo', content: <div className="p-4"><CodeBlock code="cargo install galex" language="bash" /></div> },
]

/* ── SVG Illustrations ──────────────────────────────────────────────── */

function TypeGraphSvg() {
  return (
    <svg viewBox="0 0 200 120" fill="none" className="w-full h-full opacity-80">
      <circle cx="40" cy="30" r="14" className="fill-accent/10 stroke-accent/40" strokeWidth="1" />
      <circle cx="100" cy="20" r="14" className="fill-accent/10 stroke-accent/40" strokeWidth="1" />
      <circle cx="160" cy="35" r="14" className="fill-accent/10 stroke-accent/40" strokeWidth="1" />
      <circle cx="60" cy="80" r="14" className="fill-accent/10 stroke-accent/40" strokeWidth="1" />
      <circle cx="130" cy="90" r="14" className="fill-accent/10 stroke-accent/40" strokeWidth="1" />
      <line x1="52" y1="37" x2="88" y2="23" className="stroke-accent/20" strokeWidth="1" />
      <line x1="112" y1="27" x2="148" y2="32" className="stroke-accent/20" strokeWidth="1" />
      <line x1="46" y1="43" x2="55" y2="68" className="stroke-accent/20" strokeWidth="1" />
      <line x1="106" y1="33" x2="126" y2="77" className="stroke-accent/20" strokeWidth="1" />
      <line x1="72" y1="83" x2="118" y2="88" className="stroke-accent/20" strokeWidth="1" />
      <line x1="154" y1="47" x2="136" y2="78" className="stroke-accent/20" strokeWidth="1" />
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
      <rect x="10" y="15" width="32" height="40" rx="4" className="fill-accent/8 stroke-accent/30" strokeWidth="1" />
      <rect x="50" y="15" width="32" height="40" rx="4" className="fill-accent/8 stroke-accent/30" strokeWidth="1" />
      <rect x="90" y="15" width="32" height="40" rx="4" className="fill-accent/8 stroke-accent/30" strokeWidth="1" />
      <text x="26" y="39" textAnchor="middle" className="fill-accent/60 text-[6px] font-mono">.gx</text>
      <text x="66" y="39" textAnchor="middle" className="fill-accent/60 text-[6px] font-mono">.gx</text>
      <text x="106" y="39" textAnchor="middle" className="fill-accent/60 text-[6px] font-mono">.gx</text>
      <path d="M130 35 L155 35" className="stroke-accent/30" strokeWidth="1.5" strokeLinecap="round" markerEnd="url(#ah)" />
      <defs><marker id="ah" markerWidth="6" markerHeight="4" refX="5" refY="2" orient="auto"><polygon points="0 0, 6 2, 0 4" className="fill-accent/30" /></marker></defs>
      <rect x="162" y="10" width="65" height="50" rx="6" className="fill-accent/12 stroke-accent/40" strokeWidth="1.5" />
      <text x="194" y="30" textAnchor="middle" className="fill-accent text-[7px] font-mono font-bold">gale_app</text>
      <text x="194" y="42" textAnchor="middle" className="fill-accent/40 text-[5px] font-mono">4.2 MB binary</text>
      <text x="66" y="72" textAnchor="middle" className="fill-muted-foreground text-[6px]">source files</text>
      <text x="194" y="76" textAnchor="middle" className="fill-muted-foreground text-[6px]">single binary</text>
    </svg>
  )
}

/* ── Page ─────────────────────────────────────────────────────────────── */

export function HomePage() {
  return (
    <div className="flex-1">

      {/* ── Hero: centered, word-by-word animated ──────────────────────── */}
      <section className="relative overflow-hidden">
        {/* Ambient background */}
        <div className="absolute inset-0 opacity-[0.025]" style={{
          backgroundImage: 'linear-gradient(hsl(var(--border)) 1px, transparent 1px), linear-gradient(90deg, hsl(var(--border)) 1px, transparent 1px)',
          backgroundSize: '48px 48px',
        }} />
        <div className="absolute top-[-300px] left-1/2 -translate-x-1/2 w-[800px] h-[800px] rounded-full bg-accent/[0.035] blur-[140px]" />

        <div className="relative max-w-5xl mx-auto px-4 sm:px-6 pt-24 pb-20 sm:pt-32 sm:pb-28">
          <div className="flex flex-col items-center text-center">

            {/* Alpha badge */}
            <motion.div
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.5 }}
            >
              <Badge variant="warning" className="mb-8">Alpha &mdash; early access</Badge>
            </motion.div>

            {/* Title — word by word */}
            <motion.h1
              initial={{ filter: 'blur(8px)', opacity: 0 }}
              animate={{ filter: 'blur(0px)', opacity: 1 }}
              transition={{ duration: 0.5 }}
              className="font-mono text-4xl font-bold sm:text-5xl md:text-6xl lg:text-7xl max-w-4xl mx-auto leading-tight tracking-tight"
            >
              {TITLE_WORDS.map((word, i) => (
                <motion.span
                  key={i}
                  initial={{ opacity: 0, y: 24 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ delay: 0.1 + i * 0.12, duration: 0.5 }}
                  className={`inline-block mx-1.5 sm:mx-2.5 ${
                    word === '.GX' ? 'bg-gradient-to-r from-accent to-emerald-300 bg-clip-text text-transparent' : ''
                  }`}
                >
                  {word}
                </motion.span>
              ))}
            </motion.h1>

            {/* Subtitle */}
            <motion.p
              initial={{ opacity: 0, y: 16 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 1.0, duration: 0.5 }}
              className="mx-auto mt-8 max-w-2xl text-lg sm:text-xl text-muted-foreground leading-relaxed"
            >
              GaleX is a Rust-native web language with typed server/client boundaries, guards, actions, and SSR. The compiler generates a standalone binary via Axum.
            </motion.p>

            {/* Feature labels */}
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              transition={{ delay: 1.5, duration: 0.5 }}
              className="mt-10 flex flex-wrap justify-center gap-6"
            >
              {HERO_LABELS.map((feat, i) => (
                <motion.div
                  key={feat.label}
                  initial={{ opacity: 0, y: 14 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ delay: 1.5 + i * 0.12, duration: 0.5, type: 'spring', stiffness: 120, damping: 14 }}
                  className="flex items-center gap-2 px-4"
                >
                  <feat.icon className="h-4 w-4 text-accent" strokeWidth={1.5} />
                  <span className="text-sm font-medium text-foreground/80">{feat.label}</span>
                </motion.div>
              ))}
            </motion.div>

            {/* CTA */}
            <motion.div
              initial={{ opacity: 0, y: 16 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 2.0, duration: 0.5, type: 'spring', stiffness: 120, damping: 14 }}
              className="mt-12 flex flex-wrap justify-center gap-3"
            >
              <Link to="/docs/getting-started">
                <Button variant="accent" size="lg" className="font-mono uppercase tracking-wide">
                  Get Started <ArrowRight className="ml-1 w-4 h-4" />
                </Button>
              </Link>
              <Link to="/install">
                <Button variant="outline" size="lg" className="font-mono uppercase tracking-wide">
                  Install
                </Button>
              </Link>
            </motion.div>
          </div>
        </div>
      </section>

      {/* ── Metrics strip ─────────────────────────────────────────────── */}
      <section className="border-y border-border/40 bg-card/30">
        <div className="max-w-6xl mx-auto px-4 sm:px-6 py-8">
          <div className="grid grid-cols-2 md:grid-cols-4 gap-8">
            {METRICS.map(m => (
              <div key={m.label}>
                <div className="text-3xl font-extrabold text-foreground tracking-tight font-mono">{m.value}</div>
                <div className="text-[12px] font-semibold text-accent mt-1">{m.label}</div>
                <div className="text-[11px] text-muted-foreground/50 mt-0.5">{m.detail}</div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* ── Bento Features Grid ───────────────────────────────────────── */}
      <section className="max-w-6xl mx-auto px-4 sm:px-6 py-20">
        <div className="text-center mb-12">
          <h2 className="text-3xl font-bold tracking-tight font-mono mb-3">What's Inside</h2>
          <p className="text-[14px] text-muted-foreground max-w-lg mx-auto">
            A compiler toolchain, type system, code generator, dev server, LSP, and CLI &mdash; built from scratch in Rust.
          </p>
        </div>

        <div className="grid grid-cols-6 gap-3">
          {/* Boundaries — wide with code */}
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
                    Server secrets physically cannot reach the client. 24 error codes track every binding across server, client, and shared scopes.
                  </p>
                </div>
              </div>
              <div className="rounded-lg overflow-hidden border border-border/30">
                <CodeBlock code={BOUNDARY_CODE} language="gx" />
              </div>
            </CardContent>
          </Card>

          {/* Type System — SVG */}
          <Card className="col-span-full sm:col-span-3 lg:col-span-2 overflow-hidden group hover:border-accent/30 transition-all">
            <CardContent className="pt-6">
              <div className="h-28 mb-4 flex items-center justify-center"><TypeGraphSvg /></div>
              <h3 className="text-lg font-semibold mb-1.5">Full type inference</h3>
              <p className="text-[13px] text-muted-foreground leading-relaxed">
                Constraint-based type system with Robinson unification. Signals, guards, DOM events, templates &mdash; all checked at compile time.
              </p>
            </CardContent>
          </Card>

          {/* Guards — code */}
          <Card className="col-span-full sm:col-span-3 lg:col-span-3 overflow-hidden group hover:border-accent/30 transition-all">
            <CardContent className="pt-6">
              <div className="flex items-center gap-2 mb-3">
                <div className="flex items-center justify-center w-9 h-9 rounded-full border border-border/60 bg-accent/5">
                  <FileCode className="w-4 h-4 text-accent" strokeWidth={1.5} />
                </div>
              </div>
              <h3 className="text-lg font-semibold mb-2">28 guard validators</h3>
              <p className="text-[13px] text-muted-foreground leading-relaxed mb-3">
                Typed schemas generating both Rust and JS validation. Composition via <code className="text-accent/80 bg-accent/10 px-1 rounded text-[12px]">.partial()</code>, <code className="text-accent/80 bg-accent/10 px-1 rounded text-[12px]">.pick()</code>, <code className="text-accent/80 bg-accent/10 px-1 rounded text-[12px]">.omit()</code>.
              </p>
              <div className="rounded-lg overflow-hidden border border-border/30 text-[12px]">
                <CodeBlock code={GUARD_CODE} language="gx" />
              </div>
            </CardContent>
          </Card>

          {/* Binary — SVG */}
          <Card className="col-span-full sm:col-span-3 lg:col-span-3 overflow-hidden group hover:border-accent/30 transition-all">
            <CardContent className="pt-6">
              <div className="h-24 mb-4"><BinaryOutputSvg /></div>
              <h3 className="text-lg font-semibold mb-1.5">Single binary output</h3>
              <p className="text-[13px] text-muted-foreground leading-relaxed">
                <code className="text-accent/80 bg-accent/10 px-1 rounded text-[12px]">gale build</code> compiles .gx into a standalone Rust binary. No Node.js, no runtime deps.
              </p>
            </CardContent>
          </Card>

          {/* Small feature cards */}
          {[
            { icon: Zap, title: 'SSR by default', desc: 'Server-rendered HTML. Selective hydration. Static pages ship zero JS.' },
            { icon: Terminal, title: '16 CLI commands', desc: 'new, build, dev, check, lint, serve, add, publish, and more.' },
            { icon: Eye, title: 'LSP — 10 features', desc: 'Diagnostics, hover, go-to-def, rename, references. VS Code + Zed.' },
            { icon: GitBranch, title: 'File-based routing', desc: 'Dynamic [slug] segments. Catch-all [...rest]. Auto-nested layouts.' },
            { icon: Cpu, title: 'Dev server + HMR', desc: 'WebSocket hot reload. 50ms debounce. Error overlay. Incremental.' },
            { icon: Box, title: 'Package registry', desc: 'add, remove, update, search, publish. SHA-256 verified.' },
          ].map(f => (
            <Card key={f.title} className="col-span-3 lg:col-span-1 overflow-hidden group hover:border-accent/30 transition-all">
              <CardContent className="pt-6">
                <f.icon className="w-5 h-5 text-accent mb-3 group-hover:scale-110 transition-transform" strokeWidth={1.5} />
                <h3 className="text-[14px] font-semibold mb-1">{f.title}</h3>
                <p className="text-[11px] text-muted-foreground/70 leading-relaxed">{f.desc}</p>
              </CardContent>
            </Card>
          ))}
        </div>
      </section>

      {/* ── Install ───────────────────────────────────────────────────── */}
      <section className="border-t border-border/40 bg-card/20">
        <div className="max-w-6xl mx-auto px-4 sm:px-6 py-20">
          <div className="grid lg:grid-cols-2 gap-10 items-start">
            <div>
              <h2 className="text-2xl font-bold tracking-tight mb-3">One command install</h2>
              <p className="text-[14px] text-muted-foreground leading-relaxed mb-5">
                The SDK installer puts <code className="text-accent/80 bg-accent/10 px-1 py-0.5 rounded text-[12px]">gale</code> and <code className="text-accent/80 bg-accent/10 px-1 py-0.5 rounded text-[12px]">gale-lsp</code> on your PATH. No admin rights required.
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

      {/* ── Docs grid ─────────────────────────────────────────────────── */}
      <section className="max-w-6xl mx-auto px-4 sm:px-6 py-20">
        <div className="text-center mb-12">
          <h2 className="text-2xl font-bold tracking-tight mb-3">Learn more</h2>
          <p className="text-[14px] text-muted-foreground">Guides, references, and patterns.</p>
        </div>
        <div className="grid sm:grid-cols-2 lg:grid-cols-3 gap-3">
          {[
            { title: 'Getting Started', desc: 'Install, scaffold, run the dev server.', href: '/docs/getting-started' },
            { title: 'Guards', desc: '28 validators, composition, type checks.', href: '/docs/reference/guards' },
            { title: 'Boundaries', desc: 'server/client/shared, 24 error codes.', href: '/docs/reference/boundaries' },
            { title: 'Templates', desc: 'Directives, when/each, DOM typing.', href: '/docs/reference/templates' },
            { title: 'CLI', desc: '16 commands: build, dev, check, lint.', href: '/docs/cli/project' },
            { title: 'Deploying', desc: 'Docker, health checks, systemd.', href: '/docs/guides/deploying' },
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
