import { Link, useLocation } from 'react-router-dom'
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
      { label: 'Language Reference', href: '/docs/reference' },
      { label: 'API Reference', href: '/docs/api' },
      { label: 'CLI Reference', href: '/docs/cli' },
      { label: 'Config', href: '/docs/config' },
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
  {
    title: 'More',
    links: [
      { label: 'Install', href: '/install' },
      { label: 'Examples', href: '/examples' },
      { label: 'VS Code', href: '/editors/vscode' },
      { label: 'Zed', href: '/editors/zed' },
    ],
  },
]

interface DocLayoutProps {
  children: React.ReactNode
}

export function DocLayout({ children }: DocLayoutProps) {
  const location = useLocation()

  return (
    <div className="max-w-6xl mx-auto px-4 sm:px-6 flex gap-0">
      {/* Sidebar */}
      <aside className="hidden lg:block w-56 shrink-0 py-8 pr-6">
        <nav className="sticky top-20 space-y-6">
          {SIDEBAR_SECTIONS.map(section => (
            <div key={section.title}>
              <h4 className="text-[11px] font-medium tracking-wider uppercase text-muted-foreground/50 mb-2 px-2">
                {section.title}
              </h4>
              <ul className="space-y-0.5">
                {section.links.map(link => (
                  <li key={link.href}>
                    <Link
                      to={link.href}
                      className={cn(
                        'block px-2 py-1.5 rounded-md text-[13px] transition-colors',
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
