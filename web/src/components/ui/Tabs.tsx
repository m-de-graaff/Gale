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
    <div className={cn('rounded-lg border border-border overflow-hidden', className)}>
      <div className="flex border-b border-border bg-muted/50">
        {tabs.map((tab, i) => (
          <button
            key={tab.label}
            onClick={() => setActive(i)}
            className={cn(
              'px-4 py-2 text-[12px] font-medium transition-colors relative',
              active === i
                ? 'text-foreground'
                : 'text-muted-foreground hover:text-foreground/70'
            )}
          >
            {tab.label}
            {active === i && (
              <span className="absolute bottom-0 left-0 right-0 h-px bg-accent" />
            )}
          </button>
        ))}
      </div>
      <div className="bg-[#fafafa]">
        {tabs[active]?.content}
      </div>
    </div>
  )
}
