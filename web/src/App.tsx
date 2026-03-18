import { useEffect, useState } from 'react'
import {
  Link, NavLink, Navigate, Route, Routes, useLocation,
} from 'react-router-dom'
import {
  Activity, ArrowRight, Binary, Box, ChevronRight, Code2,
  Copy, Check, GitBranch, Globe, Package, Server,
  Shield, Terminal, Zap, Map as MapIcon, MessageCircle,
  Download, ExternalLink, BookOpen, Layers, Cpu,
} from 'lucide-react'
import { Area, AreaChart, CartesianGrid } from 'recharts'
import { cn } from '@/lib/utils'
import {
  type EditorGuide, type ExampleCard,
  canonicalExamples, docsNavGroups, docsPages, downloadMatrix,
  editorGuides, getDocByPath, installTabs,
  RELEASES_URL, repoExamples, REPO_URL,
} from './content'
import { ChartContainer, ChartTooltip, ChartTooltipContent, type ChartConfig } from '@/components/ui/chart'

// ── Router ───────────────────────────────────────────────────────────────────
export default function App() {
  return (
    <>
      <ScrollToTop />
      <Routes>
        <Route path="/" element={<Shell><HomePage /></Shell>} />
        <Route path="/install" element={<Shell><InstallPage /></Shell>} />
        <Route path="/examples" element={<Shell><ExamplesPage /></Shell>} />
        <Route path="/editors/vscode" element={<Shell><EditorPage slug="vscode" /></Shell>} />
        <Route path="/editors/zed" element={<Shell><EditorPage slug="zed" /></Shell>} />
        {docsPages.map((page) => (
          <Route
            key={page.path}
            path={page.path}
            element={<Shell><DocPageView routePath={page.path} /></Shell>}
          />
        ))}
        <Route path="/docs" element={<Navigate to="/docs/getting-started" replace />} />
        <Route path="*" element={<Shell><NotFoundPage /></Shell>} />
      </Routes>
    </>
  )
}

function ScrollToTop() {
  const { pathname } = useLocation()
  useEffect(() => { window.scrollTo(0, 0) }, [pathname])
  return null
}

// ── Shell ─────────────────────────────────────────────────────────────────────
function Shell({ children }: { children: React.ReactNode }) {
  const { pathname } = useLocation()
  const isDocs = pathname.startsWith('/docs')
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false)

  return (
    <div className="flex min-h-screen flex-col bg-background text-foreground">
      {/* Nav */}
      <header className="sticky top-0 z-50 border-b border-border/60 bg-background/80 backdrop-blur-md">
        <div className="mx-auto flex h-14 max-w-[1200px] items-center gap-6 px-4 sm:px-6">
          {/* Logo */}
          <Link to="/" className="flex shrink-0 items-center gap-2.5 font-semibold tracking-tight" aria-label="Gale home">
            <WindLogo />
            <span className="text-[15px]">Gale</span>
          </Link>

          {/* Primary nav */}
          <nav className="hidden md:flex items-center gap-1 text-[13px]">
            {[
              { to: '/docs/getting-started', label: 'Docs',    active: isDocs },
              { to: '/install',              label: 'Install'                  },
              { to: '/examples',             label: 'Examples'                 },
              { to: '/editors/vscode',       label: 'Editors', active: pathname.startsWith('/editors') },
            ].map(({ to, label, active }) => (
              <NavLink
                key={to}
                to={to}
                className={({ isActive }) => cn(
                  'px-3 py-1.5 rounded-md transition-colors',
                  (isActive || active) ? 'text-foreground' : 'text-muted-foreground hover:text-foreground',
                )}
              >
                {label}
              </NavLink>
            ))}
          </nav>

          <div className="ml-auto flex items-center gap-3">
            <a
              href={REPO_URL}
              target="_blank"
              rel="noreferrer"
              className="hidden sm:flex items-center gap-1.5 text-[13px] text-muted-foreground hover:text-foreground transition-colors"
            >
              <GitBranch className="size-3.5" />
              GitHub
            </a>
            <Link
              to="/docs/getting-started"
              className="flex items-center gap-1.5 rounded-md bg-white px-3 py-1.5 text-[13px] font-medium text-black hover:bg-white/90 transition-colors"
            >
              Get Started
              <ArrowRight className="size-3" />
            </Link>
            <button
              className="md:hidden p-1.5 text-muted-foreground"
              onClick={() => setMobileMenuOpen(o => !o)}
              aria-label="Toggle menu"
            >
              {mobileMenuOpen ? '✕' : '☰'}
            </button>
          </div>
        </div>

        {/* Mobile menu */}
        {mobileMenuOpen && (
          <div className="md:hidden border-t border-border/60 bg-background px-4 py-3">
            {[
              { to: '/docs/getting-started', label: 'Docs' },
              { to: '/install', label: 'Install' },
              { to: '/examples', label: 'Examples' },
              { to: '/editors/vscode', label: 'VS Code' },
              { to: '/editors/zed', label: 'Zed' },
            ].map(({ to, label }) => (
              <NavLink
                key={to}
                to={to}
                className="block py-2 text-[13px] text-muted-foreground hover:text-foreground"
                onClick={() => setMobileMenuOpen(false)}
              >
                {label}
              </NavLink>
            ))}
          </div>
        )}
      </header>

      {/* Content */}
      <main className="flex-1">{children}</main>

      {/* Footer */}
      <footer className="border-t border-border/60 mt-auto">
        <div className="mx-auto max-w-[1200px] px-4 sm:px-6 py-10">
          <div className="grid grid-cols-2 gap-8 sm:grid-cols-4">
            <div className="col-span-2 sm:col-span-1">
              <div className="flex items-center gap-2 mb-3 font-semibold text-[15px]">
                <WindLogo />
                Gale
              </div>
              <p className="text-[13px] text-muted-foreground leading-relaxed">
                Rust-native web framework with .gx syntax, typed boundaries, and single-binary deployment.
              </p>
            </div>
            {[
              {
                title: 'Docs',
                links: [
                  { to: '/docs/getting-started', label: 'Getting Started' },
                  { to: '/docs/reference', label: 'Language Reference' },
                  { to: '/docs/api', label: 'API Reference' },
                  { to: '/docs/config', label: 'Config' },
                ],
              },
              {
                title: 'Tools',
                links: [
                  { to: '/install', label: 'Install' },
                  { to: '/editors/vscode', label: 'VS Code' },
                  { to: '/editors/zed', label: 'Zed' },
                  { href: RELEASES_URL, label: 'Releases' },
                ],
              },
              {
                title: 'Guides',
                links: [
                  { to: '/docs/guides/forms', label: 'Forms' },
                  { to: '/docs/guides/auth', label: 'Auth' },
                  { to: '/docs/guides/database', label: 'Database' },
                  { to: '/docs/guides/deploying', label: 'Deploying' },
                ],
              },
            ].map(({ title, links }) => (
              <div key={title}>
                <p className="mb-3 text-[12px] font-medium text-muted-foreground uppercase tracking-wider">{title}</p>
                <ul className="space-y-2">
                  {links.map((l) => (
                    <li key={l.label}>
                      {'to' in l && l.to ? (
                        <Link to={l.to} className="text-[13px] text-muted-foreground hover:text-foreground transition-colors">{l.label}</Link>
                      ) : 'href' in l ? (
                        <a href={l.href} target="_blank" rel="noreferrer" className="text-[13px] text-muted-foreground hover:text-foreground transition-colors">{l.label}</a>
                      ) : null}
                    </li>
                  ))}
                </ul>
              </div>
            ))}
          </div>
          <div className="mt-10 border-t border-border/60 pt-6 flex flex-col sm:flex-row sm:items-center sm:justify-between gap-2">
            <p className="text-[12px] text-muted-foreground">MIT / Apache-2.0 — Gale contributors</p>
            <a href={REPO_URL} target="_blank" rel="noreferrer" className="text-[12px] text-muted-foreground hover:text-foreground transition-colors">
              github.com/m-de-graaff/Gale
            </a>
          </div>
        </div>
      </footer>
    </div>
  )
}

// ── Wind logo SVG ─────────────────────────────────────────────────────────────
function WindLogo({ className }: { className?: string }) {
  return (
    <svg className={cn('size-5', className)} viewBox="0 0 20 20" fill="none">
      <rect x="2" y="7" width="16" height="1.8" rx="0.9" fill="currentColor"/>
      <rect x="2" y="11" width="12" height="1.8" rx="0.9" fill="currentColor" opacity="0.75"/>
      <rect x="2" y="15" width="8" height="1.8" rx="0.9" fill="currentColor" opacity="0.45"/>
    </svg>
  )
}

// ── Home page ─────────────────────────────────────────────────────────────────
function HomePage() {
  return (
    <div>
      <HeroSection />
      <FeaturesSection />
      <InstallSection />
      <ExamplesPreview />
      <DocsGrid />
    </div>
  )
}

function HeroSection() {
  return (
    <section className="relative overflow-hidden border-b border-border/60">
      {/* Background grid */}
      <div
        className="pointer-events-none absolute inset-0 opacity-[0.03]"
        style={{
          backgroundImage: 'linear-gradient(hsl(var(--foreground)) 1px, transparent 1px), linear-gradient(90deg, hsl(var(--foreground)) 1px, transparent 1px)',
          backgroundSize: '60px 60px',
        }}
      />

      <div className="mx-auto max-w-[1200px] px-4 sm:px-6 py-20 sm:py-28 lg:py-36">
        <div className="flex flex-col items-center text-center">
          {/* Badge */}
          <div className="mb-6 inline-flex items-center gap-2 rounded-full border border-border/80 bg-muted/40 px-3 py-1 text-[12px] text-muted-foreground">
            <span className="size-1.5 rounded-full bg-green-500 inline-block" />
            Framework v0.1 — early access
          </div>

          {/* Headline */}
          <h1 className="max-w-3xl text-4xl sm:text-5xl lg:text-7xl font-bold tracking-tight text-foreground" style={{ letterSpacing: '-0.04em', lineHeight: 1.05 }}>
            Write{' '}
            <span className="text-muted-foreground font-mono text-3xl sm:text-4xl lg:text-5xl px-2 py-1 rounded-lg bg-muted/60 border border-border/60">.gx</span>
            {' '}files.{' '}
            <br className="hidden sm:block" />
            Ship one binary.
          </h1>

          <p className="mt-6 max-w-xl text-[15px] text-muted-foreground leading-relaxed">
            Gale is a Rust-native web framework with typed server/client boundaries,
            server-side rendering, and a sub-3KB client runtime — compiled to a single deployable binary.
          </p>

          {/* CTAs */}
          <div className="mt-8 flex flex-wrap items-center justify-center gap-3">
            <Link
              to="/docs/getting-started"
              className="flex items-center gap-2 rounded-md bg-white px-5 py-2.5 text-[14px] font-medium text-black hover:bg-white/90 transition-colors"
            >
              <BookOpen className="size-4" />
              Get Started
            </Link>
            <Link
              to="/install"
              className="flex items-center gap-2 rounded-md border border-border/80 bg-muted/30 px-5 py-2.5 text-[14px] font-medium text-foreground hover:bg-muted/60 transition-colors"
            >
              <Download className="size-4" />
              Install
            </Link>
            <a
              href={REPO_URL}
              target="_blank"
              rel="noreferrer"
              className="flex items-center gap-2 rounded-md border border-border/80 px-5 py-2.5 text-[14px] text-muted-foreground hover:text-foreground hover:border-border transition-colors"
            >
              <GitBranch className="size-4" />
              GitHub
            </a>
          </div>

          {/* Metrics strip */}
          <div className="mt-14 grid grid-cols-2 sm:grid-cols-4 gap-px w-full max-w-2xl rounded-xl overflow-hidden border border-border/60 bg-border/60">
            {[
              { label: 'Output', value: 'Single binary' },
              { label: 'Client JS', value: '< 3 KB' },
              { label: 'Render mode', value: 'SSR default' },
              { label: 'Boundary', value: 'Compile-time' },
            ].map(({ label, value }) => (
              <div key={label} className="flex flex-col items-center gap-1 bg-background px-4 py-4">
                <span className="text-[11px] text-muted-foreground uppercase tracking-wider">{label}</span>
                <span className="text-[15px] font-semibold text-foreground">{value}</span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </section>
  )
}

// ── Features (Vercel-style bordered grid) ─────────────────────────────────────
function FeaturesSection() {
  return (
    <section className="border-b border-border/60">
      <div className="mx-auto max-w-[1200px] px-4 sm:px-6 py-16 md:py-24">
        <div className="mb-10 text-center">
          <p className="text-[13px] text-muted-foreground uppercase tracking-widest mb-2">The Gale difference</p>
          <h2 className="text-2xl sm:text-3xl font-bold tracking-tight">Built for production from day one</h2>
        </div>

        {/* Grid: top 2 cells + full-width stat + bottom chart */}
        <div className="mx-auto grid max-w-4xl border border-border/60 md:grid-cols-2">
          {/* Cell 1: Typed boundaries */}
          <div className="border-b border-border/60 md:border-r">
            <div className="p-6 sm:p-10">
              <span className="flex items-center gap-2 text-[13px] text-muted-foreground mb-6">
                <Shield className="size-4" />
                Compiler-enforced boundaries
              </span>
              <p className="text-xl font-semibold leading-snug">
                Server secrets can't reach the client. The compiler won't let them.
              </p>
            </div>
            <BoundaryVisual />
          </div>

          {/* Cell 2: Chat/DX */}
          <div className="border-b border-border/60 overflow-hidden bg-muted/10">
            <div className="relative z-10 p-6 sm:p-10">
              <span className="flex items-center gap-2 text-[13px] text-muted-foreground mb-6">
                <MessageCircle className="size-4" />
                GX-coded diagnostics
              </span>
              <p className="text-xl font-semibold leading-snug">
                Every error has a stable code. Every code has a docs page.
              </p>
            </div>
            <DiagnosticsVisual />
          </div>

          {/* Full-width stat */}
          <div className="col-span-full border-b border-border/60 py-10 px-6 text-center">
            <p className="text-5xl sm:text-7xl font-bold tracking-tight">Sub-3 KB</p>
            <p className="mt-2 text-[14px] text-muted-foreground">client runtime, gzipped — no virtual DOM, no framework overhead</p>
          </div>

          {/* Activity chart — full width */}
          <div className="col-span-full relative">
            <div className="absolute z-10 max-w-sm px-6 pt-6 sm:px-10 sm:pt-10">
              <span className="flex items-center gap-2 text-[13px] text-muted-foreground mb-4">
                <Activity className="size-4" />
                Request throughput
              </span>
              <p className="text-xl font-semibold leading-snug">
                &gt;100k req/s on a single core.{' '}
                <span className="text-muted-foreground">Axum + Tokio under the hood.</span>
              </p>
            </div>
            <ThroughputChart />
          </div>
        </div>
      </div>
    </section>
  )
}

function BoundaryVisual() {
  return (
    <div aria-hidden className="relative overflow-hidden px-6 pb-6 sm:px-10 sm:pb-10">
      <div className="rounded-lg border border-border/60 overflow-hidden text-[12px] font-mono">
        <div className="bg-muted/30 border-b border-border/60 px-3 py-1.5 flex items-center gap-2">
          <span className="size-2 rounded-full bg-red-500/70" />
          <span className="size-2 rounded-full bg-yellow-500/70" />
          <span className="size-2 rounded-full bg-green-500/70" />
          <span className="ml-2 text-muted-foreground">page.gx</span>
        </div>
        <div className="p-4 space-y-1 leading-6 bg-background">
          <div><span className="text-[#888]">server {'{'}</span></div>
          <div className="pl-4"><span className="text-green-400">let</span> <span className="text-blue-400">secret</span> = env.SESSION_SECRET</div>
          <div><span className="text-[#888]">{'}'}</span></div>
          <div className="mt-2"><span className="text-[#888]">client {'{'}</span></div>
          <div className="pl-4 flex items-start gap-2">
            <span><span className="text-red-400">// ✗</span> <span className="text-muted-foreground">secret</span> — <span className="text-red-400 text-[11px]">GX0500</span></span>
          </div>
          <div><span className="text-[#888]">{'}'}</span></div>
        </div>
      </div>
    </div>
  )
}

function DiagnosticsVisual() {
  const errors = [
    { code: 'GX0500', label: 'Server binding in client', color: 'text-red-400' },
    { code: 'GX0705', label: 'Missing key in each block', color: 'text-yellow-400' },
    { code: 'GX0300', label: 'Type mismatch: int vs float', color: 'text-red-400' },
    { code: 'GX1403', label: 'Missing head.title on page', color: 'text-yellow-400' },
  ]
  return (
    <div aria-hidden className="px-6 pb-8 sm:px-10">
      <div className="space-y-2">
        {errors.map((e) => (
          <div key={e.code} className="flex items-center gap-3 rounded-lg border border-border/60 bg-background px-3 py-2.5 text-[12px] font-mono">
            <span className={cn('font-semibold', e.color)}>{e.code}</span>
            <span className="text-muted-foreground truncate">{e.label}</span>
          </div>
        ))}
      </div>
    </div>
  )
}

const throughputConfig = {
  requests: { label: 'Requests/s', color: 'hsl(222 89% 55%)' },
  baseline: { label: 'Baseline', color: 'hsl(0 0% 30%)' },
} satisfies ChartConfig

const throughputData = [
  { t: '0s', requests: 12000, baseline: 8000 },
  { t: '5s', requests: 45000, baseline: 8200 },
  { t: '10s', requests: 78000, baseline: 8100 },
  { t: '15s', requests: 95000, baseline: 8300 },
  { t: '20s', requests: 102000, baseline: 8200 },
  { t: '25s', requests: 108000, baseline: 8400 },
  { t: '30s', requests: 112000, baseline: 8300 },
]

function ThroughputChart() {
  return (
    <ChartContainer className="h-56 sm:h-72 aspect-auto w-full" config={throughputConfig}>
      <AreaChart data={throughputData} margin={{ left: 0, right: 0, top: 60 }}>
        <defs>
          <linearGradient id="fillReq" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="var(--color-requests)" stopOpacity={0.3} />
            <stop offset="75%" stopColor="var(--color-requests)" stopOpacity={0} />
          </linearGradient>
          <linearGradient id="fillBase" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="var(--color-baseline)" stopOpacity={0.2} />
            <stop offset="75%" stopColor="var(--color-baseline)" stopOpacity={0} />
          </linearGradient>
        </defs>
        <CartesianGrid vertical={false} stroke="hsl(var(--border))" strokeOpacity={0.4} />
        <ChartTooltip cursor={false} content={<ChartTooltipContent className="bg-card border-border" />} />
        <Area strokeWidth={1.5} dataKey="baseline" type="monotone" fill="url(#fillBase)" stroke="var(--color-baseline)" dot={false} />
        <Area strokeWidth={2} dataKey="requests" type="monotone" fill="url(#fillReq)" stroke="var(--color-requests)" dot={false} />
      </AreaChart>
    </ChartContainer>
  )
}

// ── Install section ───────────────────────────────────────────────────────────
function InstallSection() {
  return (
    <section className="border-b border-border/60 py-16 md:py-24">
      <div className="mx-auto max-w-[1200px] px-4 sm:px-6">
        <div className="grid md:grid-cols-2 gap-12 items-start">
          <div>
            <p className="text-[13px] text-muted-foreground uppercase tracking-widest mb-3">Installation</p>
            <h2 className="text-2xl sm:text-3xl font-bold tracking-tight mb-4">
              One command. Both binaries.
            </h2>
            <p className="text-[14px] text-muted-foreground leading-relaxed mb-6">
              The SDK installer puts <code className="rounded bg-muted px-1.5 py-0.5 text-foreground font-mono">gale</code> and <code className="rounded bg-muted px-1.5 py-0.5 text-foreground font-mono">gale-lsp</code> on your PATH.
              The language server powers diagnostics in VS Code and Zed automatically.
            </p>
            <ul className="space-y-3">
              {[
                { icon: Terminal, text: 'gale CLI — new, dev, build, check, lint, fmt, test' },
                { icon: Code2, text: 'gale-lsp — language server for VS Code and Zed' },
                { icon: Package, text: 'No admin rights — installs to user-local bin dir' },
              ].map(({ icon: Icon, text }) => (
                <li key={text} className="flex items-start gap-3 text-[13px] text-muted-foreground">
                  <Icon className="size-4 mt-0.5 shrink-0 text-foreground/60" />
                  {text}
                </li>
              ))}
            </ul>
          </div>
          <InstallTabs />
        </div>
      </div>
    </section>
  )
}

// ── Examples preview ──────────────────────────────────────────────────────────
function ExamplesPreview() {
  return (
    <section className="border-b border-border/60 py-16 md:py-24">
      <div className="mx-auto max-w-[1200px] px-4 sm:px-6">
        <div className="flex items-end justify-between mb-10 gap-4">
          <div>
            <p className="text-[13px] text-muted-foreground uppercase tracking-widest mb-2">Examples</p>
            <h2 className="text-2xl sm:text-3xl font-bold tracking-tight">Patterns, not placeholders</h2>
          </div>
          <Link to="/examples" className="hidden sm:flex items-center gap-1.5 text-[13px] text-muted-foreground hover:text-foreground transition-colors whitespace-nowrap">
            All examples <ChevronRight className="size-3.5" />
          </Link>
        </div>
        <div className="grid sm:grid-cols-2 gap-4">
          {canonicalExamples.slice(0, 4).map((ex) => (
            <ExampleCard key={ex.name} ex={ex} />
          ))}
        </div>
        <div className="mt-6 sm:hidden text-center">
          <Link to="/examples" className="text-[13px] text-muted-foreground hover:text-foreground">
            See all examples →
          </Link>
        </div>
      </div>
    </section>
  )
}

function ExampleCard({ ex }: { ex: ExampleCard }) {
  return (
    <article className="rounded-lg border border-border/60 bg-card/40 p-5 hover:border-border transition-colors group">
      <div className="flex items-start justify-between gap-3 mb-3">
        <h3 className="text-[14px] font-semibold">{ex.name}</h3>
        <span className={cn(
          'shrink-0 rounded-full px-2 py-0.5 text-[11px] font-medium',
          ex.status === 'reference'
            ? 'bg-blue-500/10 text-blue-400 border border-blue-500/20'
            : 'bg-muted/60 text-muted-foreground border border-border/60',
        )}>
          {ex.status === 'reference' ? 'Reference' : 'Demo'}
        </span>
      </div>
      <p className="text-[13px] text-muted-foreground mb-3 leading-relaxed">{ex.summary}</p>
      <div className="flex flex-wrap gap-1.5">
        {ex.featureTags.map((t) => (
          <span key={t} className="rounded px-1.5 py-0.5 text-[11px] bg-muted/60 text-muted-foreground border border-border/40">{t}</span>
        ))}
      </div>
    </article>
  )
}

// ── Docs grid ─────────────────────────────────────────────────────────────────
function DocsGrid() {
  const cards = [
    { to: '/docs/getting-started', icon: BookOpen, title: 'Getting Started', body: 'Install, first project, first server action.' },
    { to: '/docs/reference', icon: Code2, title: 'GaleX Reference', body: 'Every keyword, directive, and boundary rule.' },
    { to: '/docs/api', icon: Server, title: 'Server Features', body: 'Actions, API routes, channels, guards, queries.' },
    { to: '/docs/guides/auth', icon: Shield, title: 'Auth Guide', body: 'Middleware, sessions, protected routes, env.' },
    { to: '/docs/guides/database', icon: Layers, title: 'Database', body: 'Typed env, actions as data layer, out api.' },
    { to: '/docs/guides/realtime', icon: Zap, title: 'Realtime', body: 'Channels, presence, reconnect, history.' },
  ]
  return (
    <section className="py-16 md:py-24 border-b border-border/60">
      <div className="mx-auto max-w-[1200px] px-4 sm:px-6">
        <div className="mb-10 text-center">
          <p className="text-[13px] text-muted-foreground uppercase tracking-widest mb-2">Documentation</p>
          <h2 className="text-2xl sm:text-3xl font-bold tracking-tight">Everything you need</h2>
        </div>
        <div className="grid sm:grid-cols-2 lg:grid-cols-3 gap-4">
          {cards.map(({ to, icon: Icon, title, body }) => (
            <Link
              key={to}
              to={to}
              className="group flex flex-col gap-3 rounded-lg border border-border/60 bg-card/40 p-5 hover:border-border transition-colors"
            >
              <div className="flex items-center gap-3">
                <div className="flex size-8 items-center justify-center rounded-md border border-border/60 bg-muted/40">
                  <Icon className="size-4 text-muted-foreground group-hover:text-foreground transition-colors" />
                </div>
                <h3 className="text-[14px] font-semibold">{title}</h3>
              </div>
              <p className="text-[13px] text-muted-foreground leading-relaxed">{body}</p>
              <span className="flex items-center gap-1 text-[12px] text-muted-foreground group-hover:text-foreground transition-colors mt-auto">
                Read docs <ChevronRight className="size-3" />
              </span>
            </Link>
          ))}
        </div>
      </div>
    </section>
  )
}

// ── Install page ──────────────────────────────────────────────────────────────
function InstallPage() {
  return (
    <div className="mx-auto max-w-[1200px] px-4 sm:px-6 py-12">
      <div className="max-w-2xl mb-12">
        <p className="text-[13px] text-muted-foreground uppercase tracking-widest mb-3">Install Gale</p>
        <h1 className="text-3xl sm:text-4xl font-bold tracking-tight mb-4">
          Get the full SDK in one step
        </h1>
        <p className="text-[15px] text-muted-foreground leading-relaxed">
          The SDK installer puts <code className="rounded bg-muted px-1.5 py-0.5 text-foreground font-mono text-[13px]">gale</code> and <code className="rounded bg-muted px-1.5 py-0.5 text-foreground font-mono text-[13px]">gale-lsp</code> in one move.
          No separate editor integration step needed.
        </p>
      </div>

      <div className="grid md:grid-cols-2 gap-8 mb-16">
        <InstallTabs />
        <div className="space-y-4">
          {[
            { icon: Terminal, title: 'gale CLI', body: 'new, dev, build, check, lint, fmt, test — the full lifecycle in one binary.' },
            { icon: Code2, title: 'gale-lsp', body: 'Language server for .gx files. Powers inline diagnostics in VS Code and Zed.' },
            { icon: Package, title: 'User-local install', body: '~/.local/bin (Unix) or %LOCALAPPDATA%\\Gale\\bin (Windows). No admin rights.' },
          ].map(({ icon: Icon, title, body }) => (
            <div key={title} className="flex gap-4 rounded-lg border border-border/60 p-4">
              <Icon className="size-5 shrink-0 text-muted-foreground mt-0.5" />
              <div>
                <h3 className="text-[14px] font-semibold mb-1">{title}</h3>
                <p className="text-[13px] text-muted-foreground">{body}</p>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Downloads table */}
      <div className="mb-12">
        <h2 className="text-xl font-bold tracking-tight mb-2">Manual downloads</h2>
        <p className="text-[13px] text-muted-foreground mb-6">
          Every release artifact is on{' '}
          <a href={RELEASES_URL} target="_blank" rel="noreferrer" className="text-foreground underline underline-offset-2">GitHub Releases</a>
          {' '}with SHA-256 checksums.
        </p>
        <div className="rounded-lg border border-border/60 overflow-hidden">
          <table className="w-full text-[13px]">
            <thead>
              <tr className="border-b border-border/60 bg-muted/30">
                <th className="px-4 py-3 text-left font-medium text-muted-foreground">Artifact</th>
                <th className="px-4 py-3 text-left font-medium text-muted-foreground hidden sm:table-cell">File</th>
                <th className="px-4 py-3 text-left font-medium text-muted-foreground hidden md:table-cell">Target</th>
                <th className="px-4 py-3 text-left font-medium text-muted-foreground">Notes</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-border/60">
              {downloadMatrix.map((row) => (
                <tr key={row.name} className="hover:bg-muted/20 transition-colors">
                  <td className="px-4 py-3 font-medium">{row.kind}</td>
                  <td className="px-4 py-3 hidden sm:table-cell">
                    <code className="text-[12px] text-muted-foreground">{row.name}</code>
                  </td>
                  <td className="px-4 py-3 text-muted-foreground hidden md:table-cell">{row.target}</td>
                  <td className="px-4 py-3 text-muted-foreground">{row.notes}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* Cargo fallback */}
      <div className="rounded-lg border border-border/60 bg-muted/20 p-6">
        <div className="flex items-start gap-4">
          <Box className="size-5 shrink-0 text-muted-foreground mt-0.5" />
          <div>
            <h3 className="text-[14px] font-semibold mb-1">Cargo fallback</h3>
            <p className="text-[13px] text-muted-foreground mb-3">
              <code className="rounded bg-muted px-1.5 py-0.5 text-foreground font-mono text-[12px]">cargo install galex</code> still works
              while package naming converges with the public Gale brand.
            </p>
            <CodeBlock language="bash" code="cargo install galex" compact />
          </div>
        </div>
      </div>
    </div>
  )
}

// ── Examples page ─────────────────────────────────────────────────────────────
function ExamplesPage() {
  return (
    <div className="mx-auto max-w-[1200px] px-4 sm:px-6 py-12">
      <div className="max-w-2xl mb-12">
        <p className="text-[13px] text-muted-foreground uppercase tracking-widest mb-3">Examples</p>
        <h1 className="text-3xl sm:text-4xl font-bold tracking-tight mb-4">
          Reference patterns for Gale server features
        </h1>
        <p className="text-[15px] text-muted-foreground leading-relaxed">
          The canonical patterns below show guards, actions, env, middleware, API routes,
          and channels working together — not static shells with placeholder data.
        </p>
      </div>

      {/* Reference patterns */}
      <h2 className="text-lg font-bold tracking-tight mb-4">Reference patterns</h2>
      <div className="space-y-4 mb-14">
        {canonicalExamples.map((ex) => (
          <article key={ex.name} className="rounded-lg border border-border/60 overflow-hidden">
            <div className="grid md:grid-cols-2">
              <div className="p-6">
                <div className="flex items-center gap-2 mb-3">
                  <span className="rounded-full bg-blue-500/10 text-blue-400 border border-blue-500/20 px-2 py-0.5 text-[11px] font-medium">
                    Reference
                  </span>
                </div>
                <h3 className="text-[16px] font-semibold mb-2">{ex.name}</h3>
                <p className="text-[13px] text-muted-foreground leading-relaxed mb-4">{ex.summary}</p>
                <div className="flex flex-wrap gap-1.5">
                  {ex.featureTags.map((t) => (
                    <span key={t} className="rounded px-1.5 py-0.5 text-[11px] bg-muted/60 text-muted-foreground border border-border/40">{t}</span>
                  ))}
                </div>
              </div>
              <div className="border-t md:border-t-0 md:border-l border-border/60 bg-muted/10">
                <CodeBlock language="gx" code={ex.code} />
              </div>
            </div>
          </article>
        ))}
      </div>

      {/* Repo demos */}
      <h2 className="text-lg font-bold tracking-tight mb-2">Current repo demos</h2>
      <p className="text-[13px] text-muted-foreground mb-6">
        These demos exist in the repo today, labeled accurately so docs don't overstate them.
      </p>
      <div className="grid sm:grid-cols-2 lg:grid-cols-3 gap-4">
        {repoExamples.map((ex) => (
          <article key={ex.name} className="rounded-lg border border-border/60 p-5">
            <span className="rounded-full bg-muted/60 text-muted-foreground border border-border/60 px-2 py-0.5 text-[11px] font-medium inline-block mb-3">Demo</span>
            <h3 className="text-[14px] font-semibold mb-2">{ex.name}</h3>
            <p className="text-[13px] text-muted-foreground mb-2">{ex.summary}</p>
            {ex.caveat && <p className="text-[12px] text-yellow-500/80 mb-3 italic">{ex.caveat}</p>}
            <div className="flex flex-wrap gap-1.5">
              {ex.featureTags.map((t) => (
                <span key={t} className="rounded px-1.5 py-0.5 text-[11px] bg-muted/60 text-muted-foreground border border-border/40">{t}</span>
              ))}
            </div>
          </article>
        ))}
      </div>
    </div>
  )
}

// ── Editor page ───────────────────────────────────────────────────────────────
function EditorPage({ slug }: { slug: 'vscode' | 'zed' }) {
  const guide = editorGuides.find((g) => g.slug === slug) as EditorGuide
  return (
    <div className="mx-auto max-w-[1200px] px-4 sm:px-6 py-12">
      <div className="grid md:grid-cols-3 gap-8">
        <div className="md:col-span-2">
          <p className="text-[13px] text-muted-foreground uppercase tracking-widest mb-3">Editor setup</p>
          <h1 className="text-3xl sm:text-4xl font-bold tracking-tight mb-4">{guide.title}</h1>
          <p className="text-[15px] text-muted-foreground leading-relaxed mb-8">{guide.summary}</p>

          <h2 className="text-lg font-bold tracking-tight mb-4">Quick install</h2>
          <ol className="space-y-3 mb-8">
            {guide.quickSteps.map((step, i) => (
              <li key={step} className="flex gap-3 text-[14px]">
                <span className="shrink-0 size-6 rounded-full border border-border/60 flex items-center justify-center text-[12px] text-muted-foreground font-mono mt-0.5">{i + 1}</span>
                <span className="text-muted-foreground leading-relaxed">{step}</span>
              </li>
            ))}
          </ol>

          <CodeBlock language="bash" code={guide.installCommand} className="mb-8" />

          <h2 className="text-lg font-bold tracking-tight mb-4">Notes</h2>
          <div className="space-y-3 mb-8">
            {guide.notes.map((note) => (
              <div key={note} className="flex gap-3 text-[13px] text-muted-foreground">
                <ChevronRight className="size-4 shrink-0 mt-0.5 text-foreground/40" />
                {note}
              </div>
            ))}
          </div>

          <h2 className="text-lg font-bold tracking-tight mb-4">Troubleshooting</h2>
          <div className="rounded-lg border border-border/60 divide-y divide-border/60">
            {guide.troubleshooting.map((item) => (
              <div key={item} className="px-4 py-3 text-[13px] text-muted-foreground">{item}</div>
            ))}
          </div>
        </div>

        <div>
          <div className="sticky top-20 rounded-lg border border-border/60 bg-card/40 p-5">
            <p className="text-[12px] text-muted-foreground uppercase tracking-widest mb-4">Download</p>
            <div className="mb-4 rounded-md border border-border/60 bg-muted/40 px-3 py-2.5">
              <code className="text-[13px] text-foreground font-mono">{guide.artifact}</code>
            </div>
            <p className="text-[12px] text-muted-foreground mb-4 leading-relaxed">
              Available on every GitHub release. Download, extract, run the install step.
            </p>
            <a
              href={RELEASES_URL}
              target="_blank"
              rel="noreferrer"
              className="flex w-full items-center justify-center gap-2 rounded-md bg-white px-4 py-2.5 text-[13px] font-medium text-black hover:bg-white/90 transition-colors"
            >
              <Download className="size-4" />
              GitHub Releases
              <ExternalLink className="size-3 ml-auto text-black/50" />
            </a>
            <div className="mt-4 pt-4 border-t border-border/60">
              <Link
                to={slug === 'vscode' ? '/editors/zed' : '/editors/vscode'}
                className="flex items-center justify-between text-[13px] text-muted-foreground hover:text-foreground transition-colors"
              >
                <span>{slug === 'vscode' ? 'Zed setup →' : 'VS Code setup →'}</span>
                <ChevronRight className="size-4" />
              </Link>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

// ── Docs page ─────────────────────────────────────────────────────────────────
function DocPageView({ routePath }: { routePath: string }) {
  const page = getDocByPath(routePath)
  const [sidebarOpen, setSidebarOpen] = useState(false)

  if (!page) return <NotFoundPage />

  return (
    <div className="mx-auto max-w-[1200px] px-4 sm:px-6 py-8">
      {/* Mobile sidebar toggle */}
      <button
        className="md:hidden mb-4 flex items-center gap-2 text-[13px] text-muted-foreground border border-border/60 rounded-md px-3 py-2"
        onClick={() => setSidebarOpen(o => !o)}
      >
        <Layers className="size-4" />
        {sidebarOpen ? 'Hide navigation' : 'Show navigation'}
      </button>

      <div className="flex gap-8 lg:gap-12">
        {/* Sidebar */}
        <aside className={cn(
          'shrink-0 w-56 hidden md:block',
          sidebarOpen && 'block fixed inset-0 z-50 bg-background overflow-y-auto p-6 md:relative md:inset-auto md:z-auto md:bg-transparent md:p-0',
        )}>
          {sidebarOpen && (
            <button className="mb-4 text-muted-foreground md:hidden" onClick={() => setSidebarOpen(false)}>✕ Close</button>
          )}
          <DocsSidebar current={routePath} onNav={() => setSidebarOpen(false)} />
        </aside>

        {/* Article */}
        <article className="min-w-0 flex-1">
          {/* Page header */}
          <div className="mb-8 pb-8 border-b border-border/60">
            <span className="text-[12px] text-muted-foreground uppercase tracking-widest mb-2 block">{page.group}</span>
            <h1 className="text-2xl sm:text-3xl font-bold tracking-tight mb-3">{page.title}</h1>
            <p className="text-[15px] text-muted-foreground leading-relaxed">{page.description}</p>
          </div>

          {/* Sections */}
          {page.sections.map((sec) => (
            <section key={sec.id} id={sec.id} className="mb-10 scroll-mt-20">
              <h2 className="text-[18px] font-bold tracking-tight mb-3">{sec.title}</h2>
              <p className="text-[14px] text-muted-foreground leading-relaxed mb-4">
                {sec.body.split('`').map((part, i) =>
                  i % 2 === 0
                    ? part
                    : <code key={i} className="rounded bg-muted px-1.5 py-0.5 text-foreground font-mono text-[12px]">{part}</code>
                )}
              </p>
              {sec.table && <DocsTable headers={sec.table.headers} rows={sec.table.rows} />}
              {sec.code && <CodeBlock language={sec.codeLanguage ?? 'gx'} code={sec.code} className="mt-4" />}
            </section>
          ))}

          {/* Related */}
          {page.related.length > 0 && (
            <div className="mt-12 pt-8 border-t border-border/60">
              <h3 className="text-[14px] font-semibold mb-4">Continue reading</h3>
              <div className="grid sm:grid-cols-2 gap-3">
                {page.related.map((path) => {
                  const rel = getDocByPath(path)
                  if (!rel) return null
                  return (
                    <Link
                      key={path}
                      to={path}
                      className="group flex items-start justify-between gap-3 rounded-lg border border-border/60 p-4 hover:border-border transition-colors"
                    >
                      <div>
                        <p className="text-[13px] font-medium mb-1">{rel.navLabel}</p>
                        <p className="text-[12px] text-muted-foreground line-clamp-2">{rel.description}</p>
                      </div>
                      <ChevronRight className="size-4 shrink-0 text-muted-foreground group-hover:text-foreground mt-0.5 transition-colors" />
                    </Link>
                  )
                })}
              </div>
            </div>
          )}
        </article>

        {/* ToC */}
        <aside className="hidden lg:block w-48 shrink-0">
          <div className="sticky top-20">
            <p className="text-[11px] font-medium text-muted-foreground uppercase tracking-widest mb-3">On this page</p>
            <nav className="space-y-1">
              {page.sections.map((sec) => (
                <a
                  key={sec.id}
                  href={`#${sec.id}`}
                  className="block text-[13px] text-muted-foreground hover:text-foreground transition-colors py-0.5 border-l-2 border-transparent hover:border-border pl-3"
                >
                  {sec.title}
                </a>
              ))}
            </nav>
          </div>
        </aside>
      </div>
    </div>
  )
}

function DocsSidebar({ current, onNav }: { current: string; onNav: () => void }) {
  return (
    <div className="space-y-6">
      {docsNavGroups.map(({ group, pages }) => (
        <div key={group}>
          <p className="text-[11px] font-medium text-muted-foreground uppercase tracking-widest mb-2">{group}</p>
          <nav className="space-y-0.5">
            {pages.map((page) => (
              <NavLink
                key={page.path}
                to={page.path}
                onClick={onNav}
                className={({ isActive }) => cn(
                  'block px-3 py-1.5 rounded-md text-[13px] transition-colors',
                  (isActive || current === page.path)
                    ? 'bg-muted text-foreground font-medium'
                    : 'text-muted-foreground hover:text-foreground hover:bg-muted/40',
                )}
              >
                {page.navLabel}
              </NavLink>
            ))}
          </nav>
        </div>
      ))}
      <div>
        <p className="text-[11px] font-medium text-muted-foreground uppercase tracking-widest mb-2">More</p>
        <nav className="space-y-0.5">
          {[
            { to: '/install', label: 'Install' },
            { to: '/examples', label: 'Examples' },
            { to: '/editors/vscode', label: 'VS Code' },
            { to: '/editors/zed', label: 'Zed' },
          ].map(({ to, label }) => (
            <NavLink
              key={to}
              to={to}
              onClick={onNav}
              className={({ isActive }) => cn(
                'block px-3 py-1.5 rounded-md text-[13px] transition-colors',
                isActive ? 'bg-muted text-foreground font-medium' : 'text-muted-foreground hover:text-foreground hover:bg-muted/40',
              )}
            >
              {label}
            </NavLink>
          ))}
        </nav>
      </div>
    </div>
  )
}

function DocsTable({ headers, rows }: { headers: string[]; rows: string[][] }) {
  return (
    <div className="rounded-lg border border-border/60 overflow-hidden">
      <table className="w-full text-[13px]">
        <thead>
          <tr className="border-b border-border/60 bg-muted/30">
            {headers.map((h) => (
              <th key={h} className="px-4 py-2.5 text-left font-medium text-muted-foreground">{h}</th>
            ))}
          </tr>
        </thead>
        <tbody className="divide-y divide-border/60">
          {rows.map((row, i) => (
            <tr key={i} className="hover:bg-muted/20 transition-colors">
              {row.map((cell, j) => (
                <td key={j} className="px-4 py-2.5 text-muted-foreground">
                  {j === 0
                    ? <code className="text-[12px] text-foreground font-mono">{cell}</code>
                    : cell}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

// ── 404 ───────────────────────────────────────────────────────────────────────
function NotFoundPage() {
  return (
    <div className="flex min-h-[60vh] flex-col items-center justify-center px-4 text-center">
      <p className="text-[13px] text-muted-foreground uppercase tracking-widest mb-4">404</p>
      <h1 className="text-3xl font-bold tracking-tight mb-3">Page not found</h1>
      <p className="text-[15px] text-muted-foreground mb-8">The page drifted out of Gale force.</p>
      <div className="flex gap-3">
        <Link to="/docs/getting-started" className="flex items-center gap-2 rounded-md bg-white px-4 py-2.5 text-[13px] font-medium text-black hover:bg-white/90">
          Docs
        </Link>
        <Link to="/install" className="flex items-center gap-2 rounded-md border border-border/60 px-4 py-2.5 text-[13px] text-muted-foreground hover:text-foreground hover:border-border">
          Install
        </Link>
      </div>
    </div>
  )
}

// ── Shared: InstallTabs ───────────────────────────────────────────────────────
function InstallTabs() {
  const [active, setActive] = useState(0)
  const tab = installTabs[active]

  return (
    <div className="rounded-lg border border-border/60 overflow-hidden bg-card/40">
      {/* Tab bar */}
      <div className="flex border-b border-border/60 bg-muted/20">
        {installTabs.map((t, i) => (
          <button
            key={t.label}
            onClick={() => setActive(i)}
            className={cn(
              'flex-1 px-4 py-2.5 text-[12px] font-medium transition-colors',
              active === i
                ? 'bg-background text-foreground border-b-2 border-white'
                : 'text-muted-foreground hover:text-foreground',
            )}
          >
            {t.label}
          </button>
        ))}
      </div>
      <div className="p-4">
        <CodeBlock language="bash" code={tab.command} compact />
        <p className="mt-3 text-[12px] text-muted-foreground">{tab.note}</p>
      </div>
    </div>
  )
}

// ── Shared: CodeBlock ─────────────────────────────────────────────────────────
function CodeBlock({
  code,
  language,
  compact = false,
  className,
}: {
  code: string
  language?: string
  compact?: boolean
  className?: string
}) {
  const [copied, setCopied] = useState(false)

  async function copy() {
    try {
      await navigator.clipboard.writeText(code)
      setCopied(true)
      setTimeout(() => setCopied(false), 1500)
    } catch { /* ignore */ }
  }

  return (
    <div className={cn('group relative rounded-lg border border-border/60 bg-[#0a0a0a] overflow-hidden', className)}>
      {language && (
        <div className="flex items-center justify-between px-4 py-2.5 border-b border-border/60">
          <span className="text-[11px] text-muted-foreground font-mono uppercase tracking-wider">{language}</span>
          <button
            onClick={copy}
            className="flex items-center gap-1.5 text-[11px] text-muted-foreground hover:text-foreground transition-colors"
          >
            {copied ? <Check className="size-3 text-green-400" /> : <Copy className="size-3" />}
            {copied ? 'Copied' : 'Copy'}
          </button>
        </div>
      )}
      <pre className={cn('overflow-x-auto font-mono text-[13px] leading-relaxed text-[#cdd5e0]', compact ? 'px-4 py-3' : 'px-5 py-4')}>
        <code>{code}</code>
      </pre>
      {!language && (
        <button
          onClick={copy}
          className="absolute top-2.5 right-2.5 opacity-0 group-hover:opacity-100 flex items-center gap-1 rounded-md border border-border/60 bg-muted/80 px-2 py-1 text-[11px] text-muted-foreground hover:text-foreground transition-all"
        >
          {copied ? <Check className="size-3 text-green-400" /> : <Copy className="size-3" />}
          {copied ? 'Copied' : 'Copy'}
        </button>
      )}
    </div>
  )
}

// suppress unused import warnings for icons not yet placed in JSX
void [Binary, Box, Cpu, Globe, MapIcon, Zap]
