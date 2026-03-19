# API Reference

Server primitives, client reactivity, and the validation system.

## Guards

Guards are typed validation schemas. They generate both server-side Rust validation structs and client-side JavaScript validation functions.

```text
guard CreateUser {
  name: string.trim().minLen(2).maxLen(50)
  email: string.trim().email()
  age: int.min(13).max(150)
  role: string.oneOf("user", "admin").default("user")
  bio: string.optional().maxLen(500)
}
```

### Validator methods

**String validators:**

- `.trim()` — strip whitespace (transform, not validation)
- `.email()` — RFC-compliant email pattern
- `.url()` — valid URL format
- `.uuid()` — UUID v4 format
- `.regex(pattern)` — custom regex match
- `.minLen(n)` — minimum character length
- `.maxLen(n)` — maximum character length
- `.nonEmpty()` — must not be empty string
- `.oneOf("a", "b", "c")` — must be one of the listed values

**Numeric validators (`int` and `float`):**

- `.min(n)` — minimum value
- `.max(n)` — maximum value
- `.range(min, max)` — min and max in one call
- `.positive()` — must be > 0
- `.nonNegative()` — must be >= 0

**Float-only validators:**

- `.integer()` — must be a whole number
- `.precision(n)` — decimal precision

**Universal validators:**

- `.optional()` — field may be omitted
- `.nullable()` — field may be null
- `.default(value)` — fallback value (type must match field type)
- `.custom(fn)` — custom validation function

### Guard composition

```text
// All fields become optional
let partial = CreateUser.partial()

// Only specified fields
let loginFields = CreateUser.pick("email", "password")

// Exclude specified fields
let publicFields = CreateUser.omit("password", "role")
```

### Compiler checks

The type checker validates guards extensively:

- Duplicate field names (GX0600)
- Unknown validator methods — 27 known methods (GX0606)
- Validator-type incompatibility, e.g. `.email()` on `int` (GX0607)
- Invalid numeric arguments to `.min()`, `.max()` (GX0608)
- Default value type mismatch (GX0609)
- Empty `.oneOf()` (GX0610)
- Duplicate `.oneOf()` variants (GX0611)
- Impossible ranges where `.min()` > `.max()` (GX0612)
- Self-referential guard cycles (GX0627)
- Cross-guard circular dependencies via DFS (GX0627)

## Actions

Server-side RPC endpoints. Declared inside `server` blocks.

```text
server {
  action createPost(data: PostForm) -> Post {
    let db = connect(env.DATABASE_URL)
    let post = db.insert("posts", data)
    return post
  }
}
```

Actions are exposed as POST endpoints. The compiler generates:

- **Rust:** An Axum POST handler with guard deserialization and validation
- **JavaScript:** A client-side RPC stub with `fetch()`, error classes (`GaleValidationError`, `GaleServerError`, `GaleNetworkError`), and query cache integration

### Compiler checks

- Must be in `server` block (GX0902)
- Duplicate action names (GX0904)
- Return type must be serializable (GX0903)
- Warning if no guard parameter (GX0912)

## API Resources — `out api`

Stable HTTP endpoints for external callers.

```text
out api Users {
  get() -> User[] {
    return db.query("SELECT * FROM users")
  }

  get[id](id: string) -> User {
    return db.query("SELECT * FROM users WHERE id = $1", id)
  }

  post(data: CreateUser) -> User {
    return db.insert("users", data)
  }

  delete[id](id: string) -> void {
    db.query("DELETE FROM users WHERE id = $1", id)
  }
}
```

Supports `get`, `post`, `put`, `patch`, `delete`. Path parameters are declared with `[name]` and are always typed as `string`. The compiler checks for duplicate handlers per HTTP method and validates return type serializability.

## Queries

Client-side reactive data fetching.

```text
client {
  query posts = "/api/posts" -> Post[]
  query user = "/api/users/{userId}" -> User
}
```

URL interpolation variables must be `string` or `int`. The compiler generates JavaScript fetch wrappers with caching, revalidation, and retry support.

## Channels

Typed WebSocket communication.

```text
channel chat(room: string) <-> ChatMessage {
  on connect {
    broadcast({ text: "User joined", type: "system" })
  }

  on receive(msg: ChatMessage) {
    // Validate and rebroadcast
    broadcast(msg)
  }

  on disconnect {
    broadcast({ text: "User left", type: "system" })
  }
}
```

### Direction modes

| Syntax | Direction | Use case |
|--------|-----------|----------|
| `->` | Server to client | Push notifications, live updates |
| `<-` | Client to server | Telemetry, input streaming |
| `<->` | Bidirectional | Chat, collaboration |

The compiler generates:

- **Rust:** WebSocket upgrade handler with typed message deserialization
- **JavaScript:** Client wrapper with auto-reconnect, typed send/receive

### Client subscription

```text
client {
  let ws = subscribe chat("general")

  effect {
    ws.on("message", fn(msg) {
      messages = [...messages, msg]
    })
  }
}
```

## Stores

Reactive state containers shared across components.

```text
store CartStore {
  signal items: CartItem[] = []
  derive total = items.reduce(fn(sum, item) { sum + item.price }, 0)
  derive count = items.length

  fn addItem(item: CartItem) {
    items = [...items, item]
  }

  fn removeItem(id: string) {
    items = items.filter(fn(i) { i.id != id })
  }

  fn clear() {
    items = []
  }
}
```

Stores are generated as JavaScript singleton modules. Signals cannot be mutated from outside the store (GX1002). The compiler warns if a store has no signals (GX1006).

## Middleware

Request interceptors applied to routes.

```text
middleware logRequest(req, next) {
  let start = Date.now()
  let response = next(req)
  let duration = Date.now() - start
  log("${req.method} ${req.path} ${response.status} ${duration}ms")
  return response
}
```

Middleware must have exactly 2 parameters — `req` and `next` (GX1301). The compiler checks that no client-side constructs appear in middleware bodies (GX1304).

Middleware targets:

- **Global** — applied to all routes
- **Path prefix** — applied to routes under a path
- **Resource** — applied to a specific API resource

## Env

Typed environment variable declarations.

```text
env {
  DATABASE_URL: string.nonEmpty()
  PORT: int.default(3000)
  JWT_SECRET: string.minLen(32)
  GALE_PUBLIC_API_URL: string
}
```

- Server-only env vars are accessible only from `server` blocks
- `GALE_PUBLIC_` prefixed vars are accessible from client code
- Server vars with `GALE_PUBLIC_` prefix trigger GX1103
- Client vars without `GALE_PUBLIC_` prefix trigger GX1102
- Missing validation chain triggers GX1106 warning

The compiler generates a typed Rust env config module with `dotenvy` for `.env` file loading and fail-fast validation on startup.

## CLI Reference

See the dedicated [CLI Reference](/docs/cli) for all 18 commands.
