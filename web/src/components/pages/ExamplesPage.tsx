import { Link } from 'react-router-dom'
import { ArrowRight, AlertTriangle } from 'lucide-react'
import { CodeBlock } from '@/components/ui/CodeBlock'
import { Badge } from '@/components/ui/Badge'

const PATTERNS = [
  {
    title: 'Guarded contact form',
    tags: ['guard', 'action', 'form:action', 'form:guard', 'bind:', 'signal'],
    code: `guard ContactForm {
  name: string.trim().minLen(2).maxLen(50)
  email: string.trim().email()
  message: string.trim().minLen(10).maxLen(500)
}

server {
  action submit(data: ContactForm) -> string {
    await send_email(data.email, data.message)
    return "Sent"
  }
}

client {
  signal result = ""
}

<form form:action={submit} form:guard={ContactForm}>
  <input bind:value={name} placeholder="Name" />
  <input bind:value={email} type="email" />
  <textarea bind:value={message} />
  <button type="submit">Send</button>
  <when result != "">
    <p>{result}</p>
  </when>
</form>`,
  },
  {
    title: 'REST API resource',
    tags: ['out api', 'guard', 'action', 'env'],
    code: `out api Orders {
  get[id](id: string) -> Order {
    let db = connect(env.DATABASE_URL)
    return db.query("SELECT * FROM orders WHERE id = $1", id)
  }

  post(data: CreateOrder) -> Order {
    let db = connect(env.DATABASE_URL)
    return db.query("INSERT INTO orders ... RETURNING *", data)
  }

  delete[id](id: string) -> void {
    let db = connect(env.DATABASE_URL)
    db.query("DELETE FROM orders WHERE id = $1", id)
  }
}

guard CreateOrder {
  product_id: string.trim().uuid()
  quantity: int.min(1).max(100)
}`,
  },
  {
    title: 'WebSocket channel',
    tags: ['channel', 'shared', 'enum', 'signal'],
    code: `shared {
  enum MessageType { Text, System, Join, Leave }
}

channel chat(room: string) <-> ChatMessage {
  on connect {
    broadcast({ type: MessageType.Join, text: "joined" })
  }

  on receive(msg: ChatMessage) {
    broadcast(msg)
  }

  on disconnect {
    broadcast({ type: MessageType.Leave, text: "left" })
  }
}

client {
  signal messages: ChatMessage[] = []
  let ws = subscribe chat("general")
}`,
  },
  {
    title: 'Protected middleware',
    tags: ['middleware', 'env', 'out ui'],
    code: `middleware requireAuth(req, next) {
  let token = req.header("Authorization")
  when token == null {
    return Response.status(401).json({ error: "Unauthorized" })
  }
  let user = verify_jwt(token, env.JWT_SECRET)
  when user == null {
    return Response.status(403).json({ error: "Forbidden" })
  }
  return next(req)
}

out ui AdminPage() {
  server {
    action deleteUser(id: string) -> void {
      let db = connect(env.DATABASE_URL)
      db.query("DELETE FROM users WHERE id = $1", id)
    }
  }

  <h1>Admin Dashboard</h1>
  // ...
}`,
  },
]

const REPO_DEMOS = [
  {
    name: 'examples/todo',
    description: 'File-based routing, layout composition, head metadata, and signal declarations.',
    caveat: 'Demonstrates .gx file structure. Signals are declared but not wired to interactive template elements.',
  },
  {
    name: 'examples/blog',
    description: 'Nested routes, layout with navigation, head metadata per page.',
    caveat: 'The most complete static example. Fully demonstrates layout composition and file-based routing. Content is SSR-only with no client interactivity.',
  },
  {
    name: 'examples/dashboard',
    description: 'Multi-page layout with sidebar, form elements, signal declarations.',
    caveat: 'UI structure and routing work. Signal values are declared but displayed as static placeholders in templates.',
  },
  {
    name: 'examples/ecommerce',
    description: 'Shared enums, action declarations, multi-page routing with layout.',
    caveat: 'The most feature-diverse example. Demonstrates shared enums and action declarations, but action bodies are empty stubs.',
  },
  {
    name: 'examples/chat',
    description: 'Page structure for a chat interface with signal declarations.',
    caveat: 'Describes channels conceptually in comments but does not declare a channel or wire up WebSocket functionality.',
  },
]

export function ExamplesPage() {
  return (
    <div className="flex-1">
      <div className="max-w-4xl mx-auto px-4 sm:px-6 py-16">
        <Badge variant="accent" className="mb-4">Examples</Badge>
        <h1 className="text-3xl font-bold tracking-tight mb-3">
          Reference patterns
        </h1>
        <p className="text-[14px] text-muted-foreground mb-12 max-w-lg">
          Canonical <code className="text-accent/80 bg-accent/10 px-1 py-0.5 rounded text-[12px]">.gx</code> patterns using syntax verified against the compiler's parser and type checker. These are not runnable demos &mdash; they show how GaleX constructs compose.
        </p>

        {/* Patterns */}
        <div className="space-y-10 mb-16">
          {PATTERNS.map(pattern => (
            <div key={pattern.title}>
              <div className="flex flex-wrap items-center gap-2 mb-3">
                <h2 className="text-lg font-semibold">{pattern.title}</h2>
                {pattern.tags.map(tag => (
                  <Badge key={tag} variant="outline" className="text-[10px]">{tag}</Badge>
                ))}
              </div>
              <CodeBlock code={pattern.code} language="gx" showLineNumbers />
            </div>
          ))}
        </div>

        {/* Repo demos */}
        <h2 className="text-xl font-bold tracking-tight mb-2">Repository examples</h2>
        <p className="text-[13px] text-muted-foreground mb-6">
          The <a href="https://github.com/m-de-graaff/Gale/tree/main/examples" target="_blank" rel="noopener noreferrer" className="text-accent hover:underline">examples/</a> directory contains starter projects that demonstrate file structure and syntax. These are structural demos with honest caveats about what's wired up.
        </p>

        <div className="space-y-3 mb-12">
          {REPO_DEMOS.map(demo => (
            <div key={demo.name} className="p-4 rounded-lg border border-border/40 bg-card/30">
              <h3 className="text-[14px] font-semibold font-mono text-foreground mb-1">{demo.name}</h3>
              <p className="text-[12px] text-muted-foreground/80 mb-2">{demo.description}</p>
              <div className="flex items-start gap-1.5 text-[11px] text-warning/80">
                <AlertTriangle className="w-3 h-3 mt-0.5 shrink-0" />
                <span>{demo.caveat}</span>
              </div>
            </div>
          ))}
        </div>

        <Link
          to="/docs/getting-started"
          className="inline-flex items-center gap-2 text-[13px] text-accent hover:underline"
        >
          Start building <ArrowRight className="w-3.5 h-3.5" />
        </Link>
      </div>
    </div>
  )
}
