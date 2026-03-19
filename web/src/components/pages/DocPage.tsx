import { useParams, Navigate } from 'react-router-dom'
import Markdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import rehypeRaw from 'rehype-raw'
import { DocLayout } from '@/components/layout/DocLayout'

// Import all docs as raw strings
import gettingStarted from '@/content/docs/getting-started.md?raw'
import refComponents from '@/content/docs/reference/components.md?raw'
import refGuards from '@/content/docs/reference/guards.md?raw'
import refActions from '@/content/docs/reference/actions.md?raw'
import refQueries from '@/content/docs/reference/queries.md?raw'
import refChannels from '@/content/docs/reference/channels.md?raw'
import refStores from '@/content/docs/reference/stores.md?raw'
import refApiRoutes from '@/content/docs/reference/api-routes.md?raw'
import refMiddleware from '@/content/docs/reference/middleware.md?raw'
import refEnv from '@/content/docs/reference/env.md?raw'
import refBoundaries from '@/content/docs/reference/boundaries.md?raw'
import refTemplates from '@/content/docs/reference/templates.md?raw'
import refTypes from '@/content/docs/reference/types.md?raw'
import refStatements from '@/content/docs/reference/statements.md?raw'
import cliProject from '@/content/docs/cli/project.md?raw'
import cliQuality from '@/content/docs/cli/quality.md?raw'
import cliPackages from '@/content/docs/cli/packages.md?raw'
import configServer from '@/content/docs/config/server.md?raw'
import configFeatures from '@/content/docs/config/features.md?raw'
import guideForms from '@/content/docs/guides/forms.md?raw'
import guideAuth from '@/content/docs/guides/auth.md?raw'
import guideDatabase from '@/content/docs/guides/database.md?raw'
import guideRealtime from '@/content/docs/guides/realtime.md?raw'
import guideDeploying from '@/content/docs/guides/deploying.md?raw'
import guideMigration from '@/content/docs/guides/migration.md?raw'

const DOCS: Record<string, string> = {
  'getting-started': gettingStarted,
  'reference/components': refComponents,
  'reference/guards': refGuards,
  'reference/actions': refActions,
  'reference/queries': refQueries,
  'reference/channels': refChannels,
  'reference/stores': refStores,
  'reference/api-routes': refApiRoutes,
  'reference/middleware': refMiddleware,
  'reference/env': refEnv,
  'reference/boundaries': refBoundaries,
  'reference/templates': refTemplates,
  'reference/types': refTypes,
  'reference/statements': refStatements,
  'cli/project': cliProject,
  'cli/quality': cliQuality,
  'cli/packages': cliPackages,
  'config/server': configServer,
  'config/features': configFeatures,
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
