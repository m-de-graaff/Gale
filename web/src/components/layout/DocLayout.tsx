import { useState } from 'react'
import { Link, useLocation } from 'react-router-dom'
import { Menu, X } from 'lucide-react'
import { cn } from '@/lib/utils'

const SIDEBAR_SECTIONS = [
  {
    title: 'Guide',
    links: [
      { label: 'Getting Started', href: '/docs/getting-started' },
    ],
  },
  {
    title: 'Reference',
    links: [
      { label: 'Components & Layouts', href: '/docs/reference/components' },
      { label: 'Guards', href: '/docs/reference/guards' },
      { label: 'Actions', href: '/docs/reference/actions' },
      { label: 'Queries', href: '/docs/reference/queries' },
      { label: 'Channels', href: '/docs/reference/channels' },
      { label: 'Stores', href: '/docs/reference/stores' },
      { label: 'API Routes', href: '/docs/reference/api-routes' },
      { label: 'Middleware', href: '/docs/reference/middleware' },
      { label: 'Env', href: '/docs/reference/env' },
      { label: 'Boundaries', href: '/docs/reference/boundaries' },
      { label: 'Templates', href: '/docs/reference/templates' },
      { label: 'Type System', href: '/docs/reference/types' },
      { label: 'Statements', href: '/docs/reference/statements' },
    ],
  },
  {
    title: 'CLI',
    links: [
      { label: 'Project Commands', href: '/docs/cli/project' },
      { label: 'Quality Tools', href: '/docs/cli/quality' },
      { label: 'Packages', href: '/docs/cli/packages' },
    ],
  },
  {
    title: 'Config',
    links: [
      { label: 'Server & Limits', href: '/docs/config/server' },
      { label: 'Features', href: '/docs/config/features' },
    ],
  },
  {
    title: 'Guides',
    links: [
      { label: 'Forms', href: '/docs/guides/forms' },
      { label: 'Authentication', href: '/docs/guides/auth' },
      { label: 'Database', href: '/docs/guides/database' },
      { label: 'Realtime', href: '/docs/guides/realtime' },
      { label: 'Deploying', href: '/docs/guides/deploying' },
      { label: 'Migration', href: '/docs/guides/migration' },
    ],
  },
]

interface DocLayoutProps {
  children: React.ReactNode
}

export function DocLayout({ children }: DocLayoutProps) {
  const location = useLocation()
  const [mobileOpen, setMobileOpen] = useState(false)

  const sidebar = (
    <nav className="space-y-5">
      {SIDEBAR_SECTIONS.map(section => (
        <div key={section.title}>
          <h4 className="text-[11px] font-medium tracking-wider uppercase text-muted-foreground/50 mb-1.5 px-2">
            {section.title}
          </h4>
          <ul className="space-y-0.5">
            {section.links.map(link => (
              <li key={link.href}>
                <Link
                  to={link.href}
                  onClick={() => setMobileOpen(false)}
                  className={cn(
                    'block px-2 py-1 rounded-md text-[13px] transition-colors',
                    location.pathname === link.href
                      ? 'text-foreground bg-muted/60 font-medium'
                      : 'text-muted-foreground hover:text-foreground hover:bg-muted/40'
                  )}
                >
                  {link.label}
                </Link>
              </li>
            ))}
          </ul>
        </div>
      ))}
    </nav>
  )

  return (
    <div className="max-w-6xl mx-auto px-4 sm:px-6 flex gap-0">
      {/* Mobile sidebar toggle */}
      <button
        onClick={() => setMobileOpen(!mobileOpen)}
        className="lg:hidden fixed bottom-4 right-4 z-50 p-3 rounded-full bg-accent text-accent-foreground shadow-lg"
      >
        {mobileOpen ? <X className="w-5 h-5" /> : <Menu className="w-5 h-5" />}
      </button>

      {/* Mobile sidebar overlay */}
      {mobileOpen && (
        <div className="lg:hidden fixed inset-0 z-40">
          <div className="absolute inset-0 bg-background/80 backdrop-blur-sm" onClick={() => setMobileOpen(false)} />
          <aside className="absolute left-0 top-0 bottom-0 w-64 bg-background border-r border-border/40 p-6 pt-20 overflow-y-auto">
            {sidebar}
          </aside>
        </div>
      )}

      {/* Desktop sidebar */}
      <aside className="hidden lg:block w-52 shrink-0 py-8 pr-4">
        <div className="sticky top-20 max-h-[calc(100vh-6rem)] overflow-y-auto">
          {sidebar}
        </div>
      </aside>

      {/* Content */}
      <main className="flex-1 min-w-0 py-8 lg:pl-6 lg:border-l lg:border-border/30">
        <div className="prose max-w-none">
          {children}
        </div>
      </main>
    </div>
  )
}
