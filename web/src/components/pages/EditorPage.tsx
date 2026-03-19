import { Link, useParams } from 'react-router-dom'
import { ArrowRight, Check, AlertTriangle } from 'lucide-react'
import { CodeBlock } from '@/components/ui/CodeBlock'
import { Badge } from '@/components/ui/Badge'

const EDITORS = {
  vscode: {
    name: 'VS Code',
    badge: 'Recommended',
    intro: 'The VS Code extension provides syntax highlighting, diagnostics, hover types, go-to-definition, and more for .gx files. It communicates with the gale-lsp binary.',
    install: [
      'Download the .vsix from GitHub Releases',
      'Open VS Code and press Ctrl+Shift+P (Cmd+Shift+P on macOS)',
      'Run "Extensions: Install from VSIX..."',
      'Select the downloaded .vsix file',
    ],
    installCode: `# Or install from the command line:
code --install-extension galex-vscode.vsix

# The SDK installer does this automatically on Windows
# if 'code' is on PATH.`,
    features: [
      { name: 'Syntax highlighting', desc: 'Full tokenization of .gx files including template HTML, directives, and boundary blocks', status: 'working' as const },
      { name: 'Diagnostics', desc: 'Real-time error reporting with stable GX error codes from the type checker and linter', status: 'working' as const },
      { name: 'Hover information', desc: 'Shows binding types, directive descriptions, and HTML tag info on hover', status: 'working' as const },
      { name: 'Go-to-definition', desc: 'Jump to the declaration of any binding, including cross-file definitions', status: 'working' as const },
      { name: 'Find references', desc: 'Find all usages of a name across components, functions, actions, and templates', status: 'working' as const },
      { name: 'Rename symbol', desc: 'Rename a binding across all its references', status: 'working' as const },
      { name: 'Code actions', desc: 'Quick fixes: add alt="" to img, prefix unused signal with _, hint for missing key on each', status: 'working' as const },
      { name: 'Document symbols', desc: 'Outline view showing components, functions, guards, stores, actions, channels, etc.', status: 'working' as const },
      { name: 'Folding ranges', desc: 'Fold components, layouts, guards, stores, and boundary blocks', status: 'working' as const },
      { name: 'Completions', desc: 'Shows all visible bindings, type names, keywords, HTML tags, and validator methods', status: 'partial' as const },
    ],
    caveats: [
      'Completions are not context-aware yet. The LSP returns all visible bindings regardless of cursor position.',
      'Formatting is disabled in the LSP due to known bugs in the formatter (drops parenthesized grouping, strips comments).',
    ],
    troubleshooting: [
      { q: 'No diagnostics appearing?', a: 'Ensure gale-lsp is on your PATH. Run "gale-lsp --version" in your terminal to check. If missing, re-run the SDK installer.' },
      { q: 'Extension not activating?', a: 'The extension activates on .gx file types. Open a .gx file and check the VS Code output panel for LSP logs.' },
      { q: 'Stale diagnostics?', a: 'The LSP re-analyzes on every file change. If diagnostics seem stuck, reload the VS Code window (Ctrl+Shift+P -> "Reload Window").' },
    ],
  },
  zed: {
    name: 'Zed',
    badge: 'Experimental',
    intro: 'The Zed extension provides syntax highlighting via a local Tree-sitter grammar and LSP integration for diagnostics, hover, and navigation.',
    install: [
      'Download the Zed extension archive from GitHub Releases',
      'Extract the archive',
      'Run the included install script for your platform',
      'Restart Zed',
    ],
    installCode: `# macOS / Linux
tar -xzf galex-zed.tar.gz
./install.sh

# The install script copies the Tree-sitter grammar
# into Zed's extension directory and patches extension.toml.`,
    features: [
      { name: 'Syntax highlighting', desc: 'Tree-sitter grammar covering all .gx syntax: declarations, statements, expressions, templates, types', status: 'working' as const },
      { name: 'Diagnostics', desc: 'Via gale-lsp integration', status: 'working' as const },
      { name: 'Hover information', desc: 'Via gale-lsp integration', status: 'working' as const },
      { name: 'Go-to-definition', desc: 'Via gale-lsp integration', status: 'working' as const },
      { name: 'Find references', desc: 'Via gale-lsp integration', status: 'working' as const },
      { name: 'Rename symbol', desc: 'Via gale-lsp integration', status: 'working' as const },
    ],
    caveats: [
      'The Zed extension is experimental. The Tree-sitter grammar may not cover every edge case.',
      'Same LSP caveats apply: completions are not context-aware, formatting is disabled.',
    ],
    troubleshooting: [
      { q: 'Grammar fails to load?', a: 'Ensure the install script completed successfully. Check that the grammar files are in Zed\'s extensions directory.' },
      { q: 'LSP not starting?', a: 'Zed needs gale-lsp on your PATH. Verify with "gale-lsp --version" in your terminal.' },
      { q: 'Linux path issues?', a: 'Some Linux distros use different Zed config paths. Check ~/.config/zed/ or ~/.local/share/zed/.' },
    ],
  },
}

export function EditorPage() {
  const { editor } = useParams<{ editor: string }>()
  const config = EDITORS[editor as keyof typeof EDITORS]

  if (!config) {
    return (
      <div className="flex-1 max-w-3xl mx-auto px-4 sm:px-6 py-16">
        <h1 className="text-2xl font-bold mb-4">Editor not found</h1>
        <p className="text-muted-foreground">
          Available editors: <Link to="/editors/vscode" className="text-accent hover:underline">VS Code</Link>, <Link to="/editors/zed" className="text-accent hover:underline">Zed</Link>
        </p>
      </div>
    )
  }

  return (
    <div className="flex-1">
      <div className="max-w-3xl mx-auto px-4 sm:px-6 py-16">
        <div className="flex items-center gap-3 mb-4">
          <Badge variant="accent">{config.name}</Badge>
          <Badge variant={config.badge === 'Recommended' ? 'accent' : 'warning'}>{config.badge}</Badge>
        </div>

        <h1 className="text-3xl font-bold tracking-tight mb-3">
          {config.name} for GaleX
        </h1>
        <p className="text-[14px] text-muted-foreground mb-10 max-w-lg">
          {config.intro}
        </p>

        {/* Installation */}
        <h2 className="text-xl font-bold tracking-tight mb-4">Installation</h2>
        <ol className="space-y-2 mb-5">
          {config.install.map((step, i) => (
            <li key={i} className="flex items-start gap-3 text-[13px]">
              <span className="flex items-center justify-center w-5 h-5 rounded-full bg-accent/15 text-accent text-[11px] font-semibold shrink-0 mt-0.5">
                {i + 1}
              </span>
              <span className="text-muted-foreground">{step}</span>
            </li>
          ))}
        </ol>
        <CodeBlock code={config.installCode} language="bash" className="mb-10" />

        {/* Features */}
        <h2 className="text-xl font-bold tracking-tight mb-4">Features</h2>
        <div className="space-y-2 mb-6">
          {config.features.map(f => (
            <div key={f.name} className="flex items-start gap-3 p-3 rounded-lg border border-border/30 bg-card/30">
              <div className="mt-0.5">
                {f.status === 'working' ? (
                  <Check className="w-4 h-4 text-accent" />
                ) : (
                  <AlertTriangle className="w-4 h-4 text-warning" />
                )}
              </div>
              <div>
                <div className="text-[13px] font-medium text-foreground">
                  {f.name}
                  {f.status === 'partial' && <span className="text-warning text-[11px] ml-2">(partial)</span>}
                </div>
                <p className="text-[12px] text-muted-foreground/70 mt-0.5">{f.desc}</p>
              </div>
            </div>
          ))}
        </div>

        {/* Caveats */}
        {config.caveats.length > 0 && (
          <div className="p-4 rounded-lg border border-warning/20 bg-warning/5 mb-10">
            <h3 className="text-[13px] font-semibold text-warning mb-2">Known limitations</h3>
            <ul className="space-y-1.5">
              {config.caveats.map((c, i) => (
                <li key={i} className="text-[12px] text-muted-foreground flex items-start gap-2">
                  <AlertTriangle className="w-3 h-3 text-warning/60 mt-0.5 shrink-0" />
                  {c}
                </li>
              ))}
            </ul>
          </div>
        )}

        {/* Troubleshooting */}
        <h2 className="text-xl font-bold tracking-tight mb-4">Troubleshooting</h2>
        <div className="space-y-3 mb-10">
          {config.troubleshooting.map((item, i) => (
            <div key={i} className="p-4 rounded-lg border border-border/30 bg-card/30">
              <h3 className="text-[13px] font-semibold text-foreground mb-1">{item.q}</h3>
              <p className="text-[12px] text-muted-foreground/70">{item.a}</p>
            </div>
          ))}
        </div>

        {/* Navigation */}
        <div className="flex flex-wrap gap-3">
          {editor === 'vscode' ? (
            <Link to="/editors/zed" className="inline-flex items-center gap-2 text-[13px] text-accent hover:underline">
              Zed extension <ArrowRight className="w-3.5 h-3.5" />
            </Link>
          ) : (
            <Link to="/editors/vscode" className="inline-flex items-center gap-2 text-[13px] text-accent hover:underline">
              VS Code extension <ArrowRight className="w-3.5 h-3.5" />
            </Link>
          )}
          <Link to="/docs/getting-started" className="inline-flex items-center gap-2 text-[13px] text-muted-foreground hover:text-foreground">
            Getting started <ArrowRight className="w-3.5 h-3.5" />
          </Link>
        </div>
      </div>
    </div>
  )
}
