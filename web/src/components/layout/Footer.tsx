import { Link } from 'react-router-dom'
import { GaleLogo } from './Header'

const FOOTER_SECTIONS = [
  {
    title: 'Docs',
    links: [
      { label: 'Getting Started', href: '/docs/getting-started' },
      { label: 'Guards', href: '/docs/reference/guards' },
      { label: 'Boundaries', href: '/docs/reference/boundaries' },
      { label: 'Templates', href: '/docs/reference/templates' },
      { label: 'CLI', href: '/docs/cli/project' },
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
    ],
  },
  {
    title: 'Tools',
    links: [
      { label: 'Install', href: '/install' },
      { label: 'VS Code', href: '/editors/vscode' },
      { label: 'Zed', href: '/editors/zed' },
      { label: 'Examples', href: '/examples' },
    ],
  },
  {
    title: 'Community',
    links: [
      { label: 'GitHub', href: 'https://github.com/m-de-graaff/Gale', external: true },
    ],
  },
]

export function Footer() {
  return (
    <footer className="border-t border-[rgba(255,255,255,0.06)]">
      <div className="max-w-6xl mx-auto px-4 sm:px-6 py-12">
        <div className="grid grid-cols-2 md:grid-cols-5 gap-8">
          {/* Brand */}
          <div className="col-span-2 md:col-span-1">
            <div className="flex items-center gap-2 text-accent mb-3">
              <GaleLogo />
              <span className="font-semibold text-[15px] text-foreground tracking-tight">Gale</span>
            </div>
            <p className="text-[12px] text-muted-foreground/70 leading-relaxed max-w-[200px]">
              Rust-native web framework. Write .gx files, ship a single binary.
            </p>
          </div>

          {/* Link columns */}
          {FOOTER_SECTIONS.map(section => (
            <div key={section.title}>
              <h4 className="text-[11px] font-medium tracking-wider uppercase text-muted-foreground/50 mb-3">
                {section.title}
              </h4>
              <ul className="flex flex-col gap-2">
                {section.links.map(link => (
                  <li key={link.href}>
                    {'external' in link && link.external ? (
                      <a
                        href={link.href}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="text-[13px] text-muted-foreground hover:text-foreground transition-colors"
                      >
                        {link.label}
                      </a>
                    ) : (
                      <Link
                        to={link.href}
                        className="text-[13px] text-muted-foreground hover:text-foreground transition-colors"
                      >
                        {link.label}
                      </Link>
                    )}
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </div>

        {/* Bottom */}
        <div className="mt-10 pt-6 border-t border-[rgba(255,255,255,0.06)] flex flex-col sm:flex-row items-center justify-between gap-3">
          <p className="text-[11px] text-muted-foreground/50">
            MIT / Apache-2.0 &mdash; Gale contributors
          </p>
          <a
            href="https://github.com/m-de-graaff/Gale"
            target="_blank"
            rel="noopener noreferrer"
            className="text-[11px] text-muted-foreground/50 hover:text-muted-foreground transition-colors"
          >
            github.com/m-de-graaff/Gale
          </a>
        </div>
      </div>
    </footer>
  )
}
