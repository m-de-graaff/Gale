import { cn } from '@/lib/utils'

type BadgeVariant = 'default' | 'accent' | 'warning' | 'outline'

interface BadgeProps {
  variant?: BadgeVariant
  children: React.ReactNode
  className?: string
}

const variants: Record<BadgeVariant, string> = {
  default: 'bg-muted text-muted-foreground border-border/60',
  accent: 'bg-accent/10 text-accent border-accent/25',
  warning: 'bg-warning/10 text-warning border-warning/25',
  outline: 'bg-transparent text-muted-foreground border-border',
}

export function Badge({ variant = 'default', children, className }: BadgeProps) {
  return (
    <span className={cn(
      'inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-[11px] font-medium border tracking-wide uppercase',
      variants[variant],
      className
    )}>
      {children}
    </span>
  )
}
