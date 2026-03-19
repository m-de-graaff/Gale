import { Link } from 'react-router-dom'
import { Download, Terminal, Shield, ArrowRight } from 'lucide-react'
import { CodeBlock } from '@/components/ui/CodeBlock'
import { Badge } from '@/components/ui/Badge'
import { Tabs } from '@/components/ui/Tabs'

const INSTALL_TABS = [
  {
    label: 'macOS / Linux',
    content: (
      <div className="p-4 space-y-3">
        <CodeBlock code="curl -fsSL https://get-gale.dev/install.sh | sh" language="bash" />
        <p className="text-[12px] text-muted-foreground/60 px-1">
          Detects your OS and architecture, downloads from GitHub Releases, verifies SHA-256 checksum, installs to <code className="bg-muted px-1 rounded">~/.local/bin</code> or <code className="bg-muted px-1 rounded">~/.gale/bin</code>, and patches your shell profile for PATH.
        </p>
      </div>
    ),
  },
  {
    label: 'Windows',
    content: (
      <div className="p-4 space-y-3">
        <CodeBlock code="irm https://get-gale.dev/install.ps1 | iex" language="bash" />
        <p className="text-[12px] text-muted-foreground/60 px-1">
          Downloads the Windows x86_64 SDK, verifies checksum, installs to <code className="bg-muted px-1 rounded">%LOCALAPPDATA%\Gale\bin</code>, updates your User PATH, and auto-installs the VS Code extension if <code className="bg-muted px-1 rounded">code</code> is on PATH.
        </p>
      </div>
    ),
  },
  {
    label: 'Cargo',
    content: (
      <div className="p-4 space-y-3">
        <CodeBlock code="cargo install galex" language="bash" />
        <p className="text-[12px] text-muted-foreground/60 px-1">
          Builds from source. Requires a Rust toolchain. Installs the <code className="bg-muted px-1 rounded">gale</code> CLI binary. The LSP binary is not included via cargo &mdash; use the SDK installer for the full toolchain.
        </p>
      </div>
    ),
  },
]

const DOWNLOADS = [
  { platform: 'Linux x86_64', file: 'gale-sdk-x86_64-unknown-linux-musl.tar.gz', os: 'linux' },
  { platform: 'macOS Apple Silicon', file: 'gale-sdk-aarch64-apple-darwin.tar.gz', os: 'macos' },
  { platform: 'macOS Intel', file: 'gale-sdk-x86_64-apple-darwin.tar.gz', os: 'macos' },
  { platform: 'Windows x86_64', file: 'gale-sdk-x86_64-pc-windows-msvc.zip', os: 'windows' },
]

const EXTENSIONS = [
  { platform: 'VS Code extension', file: 'galex-vscode.vsix' },
  { platform: 'Zed extension', file: 'galex-zed.tar.gz' },
  { platform: 'Checksums', file: 'checksums.txt' },
]

export function InstallPage() {
  return (
    <div className="flex-1">
      <div className="max-w-3xl mx-auto px-4 sm:px-6 py-16">
        <Badge variant="accent" className="mb-4">Install</Badge>
        <h1 className="text-3xl font-bold tracking-tight mb-3">
          Get the full SDK in one step
        </h1>
        <p className="text-[14px] text-muted-foreground mb-10 max-w-lg">
          The installer downloads <strong className="text-foreground">gale</strong> (CLI) and <strong className="text-foreground">gale-lsp</strong> (language server) for your platform.
        </p>

        {/* Install tabs */}
        <Tabs tabs={INSTALL_TABS} className="mb-12" />

        {/* What you get */}
        <div className="grid sm:grid-cols-3 gap-4 mb-12">
          <div className="p-4 rounded-lg border border-border/40 bg-card/40">
            <Terminal className="w-4 h-4 text-accent mb-2" />
            <h3 className="text-[13px] font-semibold mb-1">gale CLI</h3>
            <p className="text-[12px] text-muted-foreground/70">
              new, dev, build, check, lint, serve, add, remove, update, search, publish, login, self-update, editor
            </p>
          </div>
          <div className="p-4 rounded-lg border border-border/40 bg-card/40">
            <Shield className="w-4 h-4 text-accent mb-2" />
            <h3 className="text-[13px] font-semibold mb-1">gale-lsp</h3>
            <p className="text-[12px] text-muted-foreground/70">
              Diagnostics, hover, go-to-definition, references, rename, code actions, symbols, folding
            </p>
          </div>
          <div className="p-4 rounded-lg border border-border/40 bg-card/40">
            <Download className="w-4 h-4 text-accent mb-2" />
            <h3 className="text-[13px] font-semibold mb-1">User-local install</h3>
            <p className="text-[12px] text-muted-foreground/70">
              No admin or root required. Installs to your user directory and patches your shell profile for PATH.
            </p>
          </div>
        </div>

        {/* Manual downloads */}
        <h2 className="text-xl font-bold tracking-tight mb-4">Manual downloads</h2>
        <p className="text-[13px] text-muted-foreground mb-5">
          All artifacts are published on <a href="https://github.com/m-de-graaff/Gale/releases" target="_blank" rel="noopener noreferrer" className="text-accent hover:underline">GitHub Releases</a> with SHA-256 checksums.
        </p>

        {/* SDK table */}
        <div className="rounded-lg border border-border/40 overflow-hidden mb-6">
          <table className="w-full text-[13px]">
            <thead>
              <tr className="border-b border-border/40 bg-card/60">
                <th className="text-left px-4 py-2.5 font-medium text-muted-foreground">Platform</th>
                <th className="text-left px-4 py-2.5 font-medium text-muted-foreground">Artifact</th>
              </tr>
            </thead>
            <tbody>
              {DOWNLOADS.map(d => (
                <tr key={d.file} className="border-b border-border/20 last:border-0 hover:bg-muted/20 transition-colors">
                  <td className="px-4 py-2.5 text-foreground">{d.platform}</td>
                  <td className="px-4 py-2.5 font-mono text-[12px] text-muted-foreground">{d.file}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        {/* Extensions table */}
        <div className="rounded-lg border border-border/40 overflow-hidden mb-12">
          <table className="w-full text-[13px]">
            <thead>
              <tr className="border-b border-border/40 bg-card/60">
                <th className="text-left px-4 py-2.5 font-medium text-muted-foreground">Extension</th>
                <th className="text-left px-4 py-2.5 font-medium text-muted-foreground">Artifact</th>
              </tr>
            </thead>
            <tbody>
              {EXTENSIONS.map(e => (
                <tr key={e.file} className="border-b border-border/20 last:border-0 hover:bg-muted/20 transition-colors">
                  <td className="px-4 py-2.5 text-foreground">{e.platform}</td>
                  <td className="px-4 py-2.5 font-mono text-[12px] text-muted-foreground">{e.file}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        {/* Next steps */}
        <div className="flex flex-col sm:flex-row gap-3">
          <Link
            to="/docs/getting-started"
            className="inline-flex items-center gap-2 px-5 py-2.5 rounded-lg bg-accent text-accent-foreground text-[13px] font-semibold hover:bg-accent/90 transition-colors"
          >
            Get Started <ArrowRight className="w-3.5 h-3.5" />
          </Link>
          <Link
            to="/editors/vscode"
            className="inline-flex items-center gap-2 px-5 py-2.5 rounded-lg border border-border/60 text-[13px] font-medium hover:bg-muted/50 transition-colors"
          >
            Set up your editor
          </Link>
        </div>
      </div>
    </div>
  )
}
