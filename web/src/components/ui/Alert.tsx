import * as React from 'react'
import { cn } from '@/lib/utils'
import { AlertTriangle, Info, AlertCircle } from 'lucide-react'

type AlertVariant = 'info' | 'warning' | 'destructive'

interface AlertProps extends React.HTMLAttributes<HTMLDivElement> {
  variant?: AlertVariant
  title?: string
}

const variantStyles: Record<AlertVariant, string> = {
  info: 'border-accent/20 bg-accent/5',
  warning: 'border-warning/20 bg-warning/5',
  destructive: 'border-destructive/20 bg-destructive/5',
}

const icons: Record<AlertVariant, React.ReactNode> = {
  info: <Info className="w-4 h-4 text-accent shrink-0" />,
  warning: <AlertTriangle className="w-4 h-4 text-warning shrink-0" />,
  destructive: <AlertCircle className="w-4 h-4 text-destructive shrink-0" />,
}

const Alert = React.forwardRef<HTMLDivElement, AlertProps>(
  ({ className, variant = 'info', title, children, ...props }, ref) => (
    <div
      ref={ref}
      className={cn('flex gap-3 rounded-lg border p-4', variantStyles[variant], className)}
      {...props}
    >
      <div className="mt-0.5">{icons[variant]}</div>
      <div className="flex-1 min-w-0">
        {title && <h4 className="text-[13px] font-semibold mb-1">{title}</h4>}
        <div className="text-[12px] text-muted-foreground leading-relaxed">{children}</div>
      </div>
    </div>
  )
)
Alert.displayName = 'Alert'

export { Alert }
