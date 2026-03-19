import { useState } from 'react'
import { Link, useLocation } from 'react-router-dom'
import { Menu, X, ExternalLink, Search } from 'lucide-react'
import { cn } from '@/lib/utils'
import { Badge } from '@/components/ui/Badge'

const NAV_LINKS = [
  { label: 'Docs', href: '/docs/getting-started' },
  { label: 'Install', href: '/install' },
  { label: 'Examples', href: '/examples' },
  { label: 'Editors', href: '/editors/vscode' },
]

function GaleLogo() {
  return (
    <svg width="22" height="22" viewBox="0 0 32 32" fill="none" xmlns="http://www.w3.org/2000/svg">
      <path d="M4 10.5 C4 10.5 12 7 24 12 C24 12 16 13.5 10 17" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" opacity="0.95"/>
      <path d="M4 16.5 C4 16.5 11 14 22 18 C22 18 14 19.5 9 22" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" opacity="0.6"/>
      <path d="M4 22.5 C4 22.5 10 21 18 24" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" opacity="0.32"/>
    </svg>
  )
}

export { GaleLogo }

export function Header() {
  const [mobileOpen, setMobileOpen] = useState(false)
  const location = useLocation()

  const isActive = (href: string) => {
    if (href.startsWith('/docs')) return location.pathname.startsWith('/docs')
    if (href.startsWith('/editors')) return location.pathname.startsWith('/editors')
    return location.pathname === href
  }

  return (
    <header className="sticky top-0 z-50 border-b border-border bg-white/80 backdrop-blur-lg">
      <div className="max-w-6xl mx-auto px-4 sm:px-6 h-14 flex items-center justify-between">
        <Link to="/" className="flex items-center gap-2.5 text-accent hover:opacity-80 transition-opacity">
          <GaleLogo />
          <span className="font-semibold text-[15px] text-foreground tracking-tight">Gale</span>
          <Badge variant="warning" className="hidden sm:inline-flex">Alpha</Badge>
        </Link>

        <nav className="hidden md:flex items-center gap-1">
          {NAV_LINKS.map(link => (
            <Link
              key={link.href}
              to={link.href}
              className={cn(
                'px-3 py-1.5 rounded-md text-[13px] transition-colors',
                isActive(link.href)
                  ? 'text-foreground font-medium'
                  : 'text-muted-foreground hover:text-foreground'
              )}
            >
              {link.label}
            </Link>
          ))}
          <a
            href="https://github.com/m-de-graaff/Gale"
            target="_blank"
            rel="noopener noreferrer"
            className="px-3 py-1.5 rounded-md text-[13px] text-muted-foreground hover:text-foreground transition-colors inline-flex items-center gap-1.5"
          >
            GitHub
            <ExternalLink className="w-3 h-3" />
          </a>
          <div className="ml-2 flex items-center gap-1.5 px-2.5 py-1 rounded-md border border-border bg-muted/50 text-muted-foreground/50 cursor-default select-none">
            <Search className="w-3 h-3" />
            <span className="text-[11px] font-mono">⌘K</span>
          </div>
        </nav>

        <button
          onClick={() => setMobileOpen(!mobileOpen)}
          className="md:hidden p-2 text-muted-foreground hover:text-foreground"
        >
          {mobileOpen ? <X className="w-5 h-5" /> : <Menu className="w-5 h-5" />}
        </button>
      </div>

      {mobileOpen && (
        <div className="md:hidden border-t border-border bg-white/95 backdrop-blur-lg">
          <nav className="max-w-6xl mx-auto px-4 py-3 flex flex-col gap-1">
            {NAV_LINKS.map(link => (
              <Link
                key={link.href}
                to={link.href}
                onClick={() => setMobileOpen(false)}
                className={cn(
                  'px-3 py-2 rounded-md text-[13px] transition-colors',
                  isActive(link.href)
                    ? 'text-foreground font-medium'
                    : 'text-muted-foreground hover:text-foreground'
                )}
              >
                {link.label}
              </Link>
            ))}
            <a
              href="https://github.com/m-de-graaff/Gale"
              target="_blank"
              rel="noopener noreferrer"
              className="px-3 py-2 rounded-md text-[13px] text-muted-foreground hover:text-foreground transition-colors inline-flex items-center gap-1.5"
            >
              GitHub <ExternalLink className="w-3 h-3" />
            </a>
          </nav>
        </div>
      )}
    </header>
  )
}
