import { Link } from 'react-router-dom'
import { ArrowRight } from 'lucide-react'
import { Button } from '@/components/ui/Button'

export function NotFoundPage() {
  return (
    <div className="flex-1 flex items-center justify-center px-4 py-20">
      <div className="text-center max-w-md">
        <div className="text-6xl font-bold text-muted-foreground/20 mb-4">404</div>
        <h1 className="text-xl font-bold tracking-tight mb-2">Page not found</h1>
        <p className="text-[14px] text-muted-foreground mb-6">
          This page doesn't exist or has been moved.
        </p>
        <div className="flex justify-center gap-3">
          <Link to="/docs/getting-started">
            <Button variant="primary">Docs <ArrowRight className="w-3.5 h-3.5" /></Button>
          </Link>
          <Link to="/">
            <Button variant="outline">Home</Button>
          </Link>
        </div>
      </div>
    </div>
  )
}
