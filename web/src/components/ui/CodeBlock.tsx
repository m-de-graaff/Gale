import { useState } from 'react'
import { Check, Copy } from 'lucide-react'
import { cn } from '@/lib/utils'

interface CodeBlockProps {
  code: string
  language?: string
  filename?: string
  className?: string
  showLineNumbers?: boolean
}

export function CodeBlock({ code, language, filename, className, showLineNumbers = false }: CodeBlockProps) {
  const [copied, setCopied] = useState(false)

  const handleCopy = () => {
    navigator.clipboard.writeText(code)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  const lines = code.trimEnd().split('\n')

  return (
    <div className={cn('group relative rounded-lg border border-border/60 bg-[#0a0d12] overflow-hidden', className)}>
      {(filename || language) && (
        <div className="flex items-center justify-between px-4 py-2 border-b border-border/40 bg-[#0c1017]">
          <span className="text-[11px] font-mono text-muted-foreground/70">
            {filename || language}
          </span>
          <button
            onClick={handleCopy}
            className="flex items-center gap-1.5 text-[11px] text-muted-foreground/50 hover:text-foreground/80 transition-colors"
          >
            {copied ? <Check className="w-3 h-3" /> : <Copy className="w-3 h-3" />}
            {copied ? 'Copied' : 'Copy'}
          </button>
        </div>
      )}
      {!filename && !language && (
        <button
          onClick={handleCopy}
          className="absolute top-2.5 right-2.5 p-1.5 rounded-md text-muted-foreground/40 hover:text-foreground/70 hover:bg-white/5 transition-all opacity-0 group-hover:opacity-100"
        >
          {copied ? <Check className="w-3.5 h-3.5" /> : <Copy className="w-3.5 h-3.5" />}
        </button>
      )}
      <pre className="px-4 py-3 overflow-x-auto text-[13px] leading-[1.7]">
        <code>
          {lines.map((line, i) => (
            <span key={i} className="block">
              {showLineNumbers && (
                <span className="inline-block w-8 text-right mr-4 text-muted-foreground/30 select-none">
                  {i + 1}
                </span>
              )}
              {highlightLine(line, language)}
            </span>
          ))}
        </code>
      </pre>
    </div>
  )
}

function highlightLine(line: string, language?: string): React.ReactNode {
  if (!language || language === 'text') return line
  if (language === 'bash' || language === 'sh' || language === 'shell') return highlightBash(line)
  if (language === 'gx' || language === 'galex') return highlightGx(line)
  if (language === 'toml') return highlightToml(line)
  return line
}

function highlightBash(line: string): React.ReactNode {
  const trimmed = line.trim()
  if (trimmed.startsWith('#')) return <span className="text-emerald-700/70">{line}</span>
  if (trimmed.startsWith('$')) {
    return (
      <>
        <span className="text-muted-foreground/50">$ </span>
        <span className="text-emerald-300/90">{trimmed.slice(2)}</span>
      </>
    )
  }
  return <span className="text-foreground/80">{line}</span>
}

function highlightGx(line: string): React.ReactNode {
  const trimmed = line.trim()

  if (trimmed.startsWith('//')) return <span className="text-muted-foreground/50">{line}</span>

  const parts: React.ReactNode[] = []
  let remaining = line

  const keywords = ['out', 'ui', 'layout', 'guard', 'action', 'query', 'store', 'channel', 'middleware', 'env', 'server', 'client', 'shared', 'fn', 'let', 'mut', 'signal', 'derive', 'frozen', 'ref', 'effect', 'watch', 'when', 'each', 'else', 'return', 'if', 'for', 'in', 'from', 'use', 'type', 'enum', 'test', 'suspend', 'async', 'await', 'export', 'out api', 'true', 'false', 'null']
  const types = ['string', 'int', 'float', 'bool', 'void']
  const validators = ['.email()', '.min(', '.max(', '.minLen(', '.maxLen(', '.trim()', '.url()', '.uuid()', '.regex(', '.optional()', '.nonEmpty()', '.positive()', '.default(']

  let key = 0
  while (remaining.length > 0) {
    let matched = false

    // String literals
    const strMatch = remaining.match(/^("[^"]*"|'[^']*')/)
    if (strMatch) {
      parts.push(<span key={key++} className="text-amber-300/80">{strMatch[0]}</span>)
      remaining = remaining.slice(strMatch[0].length)
      matched = true
      continue
    }

    // Comments
    if (remaining.startsWith('//')) {
      parts.push(<span key={key++} className="text-muted-foreground/50">{remaining}</span>)
      remaining = ''
      matched = true
      continue
    }

    // Numbers
    const numMatch = remaining.match(/^\b\d+(\.\d+)?\b/)
    if (numMatch) {
      parts.push(<span key={key++} className="text-purple-300/80">{numMatch[0]}</span>)
      remaining = remaining.slice(numMatch[0].length)
      matched = true
      continue
    }

    // Validators
    for (const v of validators) {
      if (remaining.startsWith(v)) {
        parts.push(<span key={key++} className="text-cyan-300/70">{v}</span>)
        remaining = remaining.slice(v.length)
        matched = true
        break
      }
    }
    if (matched) continue

    // HTML-like tags
    const tagMatch = remaining.match(/^(<\/?[a-z][a-z0-9-]*|>|\/>)/)
    if (tagMatch) {
      parts.push(<span key={key++} className="text-rose-300/70">{tagMatch[0]}</span>)
      remaining = remaining.slice(tagMatch[0].length)
      matched = true
      continue
    }

    // Directives
    const dirMatch = remaining.match(/^(bind:|on:|class:|ref:|form:|transition:|key=|into:|prefetch=)/)
    if (dirMatch) {
      parts.push(<span key={key++} className="text-cyan-300/80">{dirMatch[0]}</span>)
      remaining = remaining.slice(dirMatch[0].length)
      matched = true
      continue
    }

    // Keywords and types
    const wordMatch = remaining.match(/^\b[a-zA-Z_]\w*\b/)
    if (wordMatch) {
      const word = wordMatch[0]
      if (keywords.includes(word)) {
        parts.push(<span key={key++} className="text-violet-400/90">{word}</span>)
      } else if (types.includes(word)) {
        parts.push(<span key={key++} className="text-emerald-400/80">{word}</span>)
      } else {
        parts.push(<span key={key++} className="text-foreground/85">{word}</span>)
      }
      remaining = remaining.slice(word.length)
      matched = true
      continue
    }

    // Template interpolation
    const interpMatch = remaining.match(/^\{[^}]*\}/)
    if (interpMatch) {
      parts.push(<span key={key++} className="text-amber-200/70">{interpMatch[0]}</span>)
      remaining = remaining.slice(interpMatch[0].length)
      matched = true
      continue
    }

    // Operators and punctuation
    parts.push(<span key={key++} className="text-muted-foreground/60">{remaining[0]}</span>)
    remaining = remaining.slice(1)
  }

  return <>{parts}</>
}

function highlightToml(line: string): React.ReactNode {
  const trimmed = line.trim()
  if (trimmed.startsWith('#')) return <span className="text-muted-foreground/50">{line}</span>
  if (trimmed.startsWith('[')) return <span className="text-violet-400/90">{line}</span>

  const eqIdx = line.indexOf('=')
  if (eqIdx > 0) {
    const key = line.slice(0, eqIdx)
    const rest = line.slice(eqIdx)
    return (
      <>
        <span className="text-emerald-400/80">{key}</span>
        <span className="text-muted-foreground/60">{rest.charAt(0)}</span>
        <span className="text-amber-300/80">{rest.slice(1)}</span>
      </>
    )
  }
  return <span className="text-foreground/80">{line}</span>
}
