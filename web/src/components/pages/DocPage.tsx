import { useParams, Navigate } from 'react-router-dom'
import Markdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import rehypeRaw from 'rehype-raw'
import { DocLayout } from '@/components/layout/DocLayout'

// Import all docs as raw strings
import gettingStarted from '@/content/docs/getting-started.md?raw'
import reference from '@/content/docs/reference.md?raw'
import api from '@/content/docs/api.md?raw'
import cli from '@/content/docs/cli.md?raw'
import config from '@/content/docs/config.md?raw'
import guideForms from '@/content/docs/guides/forms.md?raw'
import guideAuth from '@/content/docs/guides/auth.md?raw'
import guideDatabase from '@/content/docs/guides/database.md?raw'
import guideRealtime from '@/content/docs/guides/realtime.md?raw'
import guideDeploying from '@/content/docs/guides/deploying.md?raw'
import guideMigration from '@/content/docs/guides/migration.md?raw'

const DOCS: Record<string, string> = {
  'getting-started': gettingStarted,
  'reference': reference,
  'api': api,
  'cli': cli,
  'config': config,
  'guides/forms': guideForms,
  'guides/auth': guideAuth,
  'guides/database': guideDatabase,
  'guides/realtime': guideRealtime,
  'guides/deploying': guideDeploying,
  'guides/migration': guideMigration,
}

export function DocPage() {
  const { '*': slug } = useParams()

  if (!slug) return <Navigate to="/docs/getting-started" replace />

  const content = DOCS[slug]

  if (!content) {
    return (
      <DocLayout>
        <h1>Page not found</h1>
        <p>The documentation page <code>{slug}</code> does not exist.</p>
      </DocLayout>
    )
  }

  return (
    <DocLayout>
      <Markdown remarkPlugins={[remarkGfm]} rehypePlugins={[rehypeRaw]}>
        {content}
      </Markdown>
    </DocLayout>
  )
}
