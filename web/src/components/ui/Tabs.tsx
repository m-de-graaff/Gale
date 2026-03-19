import { useState } from 'react'
import { cn } from '@/lib/utils'

interface Tab {
  label: string
  content: React.ReactNode
}

interface TabsProps {
  tabs: Tab[]
  className?: string
}

export function Tabs({ tabs, className }: TabsProps) {
  const [active, setActive] = useState(0)

  return (
    <div className={cn('rounded-lg border border-border/60 overflow-hidden', className)}>
      <div className="flex border-b border-border/40 bg-[#0c1017]">
        {tabs.map((tab, i) => (
          <button
            key={tab.label}
            onClick={() => setActive(i)}
            className={cn(
              'px-4 py-2 text-[12px] font-medium transition-colors relative',
              active === i
                ? 'text-foreground'
                : 'text-muted-foreground/60 hover:text-muted-foreground'
            )}
          >
            {tab.label}
            {active === i && (
              <span className="absolute bottom-0 left-0 right-0 h-px bg-accent" />
            )}
          </button>
        ))}
      </div>
      <div className="bg-[#0a0d12]">
        {tabs[active]?.content}
      </div>
    </div>
  )
}
