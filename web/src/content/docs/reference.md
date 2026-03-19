# Language Reference

This documents every construct in the GaleX `.gx` language, verified against the compiler's AST, parser, and type checker.

## Top-level declarations

### Components — `out ui`

```text
out ui PageName(props) {
  head { ... }
  // statements
  <template>...</template>
}
```

Every page or reusable component. Props are typed parameters. The `head` block sets HTML metadata. The template is the component's HTML output.

### Layouts — `out layout`

```text
out layout RootLayout() {
  <html>
    <body>
      <nav>...</nav>
      <slot/>
    </body>
  </html>
}
```

Layouts wrap pages and must contain a `<slot/>` element (the compiler validates this). Layouts nest automatically based on file-system structure.

### Guards

```text
guard LoginForm {
  email: string.trim().email()
  password: string.minLen(8).maxLen(128)
  remember: bool.optional().default(false)
}
```

Typed validation schemas. The compiler checks validator-type compatibility:

| Validator | Works on | Error if misused |
|-----------|----------|-----------------|
| `.trim()` | `string` | GX0607 |
| `.email()` | `string` | GX0607 |
| `.url()` | `string` | GX0607 |
| `.uuid()` | `string` | GX0607 |
| `.regex(pattern)` | `string` | GX0607 |
| `.minLen(n)` | `string` | GX0607 |
| `.maxLen(n)` | `string` | GX0607 |
| `.nonEmpty()` | `string` | GX0607 |
| `.oneOf(...)` | `string` | GX0607 |
| `.min(n)` | `int`, `float` | GX0607 |
| `.max(n)` | `int`, `float` | GX0607 |
| `.positive()` | `int`, `float` | GX0607 |
| `.nonNegative()` | `int`, `float` | GX0607 |
| `.integer()` | `float` | GX0607 |
| `.range(a, b)` | `int`, `float` | GX0607 |
| `.precision(n)` | `float` | GX0607 |
| `.optional()` | any | — |
| `.nullable()` | any | — |
| `.default(v)` | any | GX0609 (type mismatch) |
| `.custom(fn)` | any | — |

The compiler also detects: duplicate fields (GX0600), impossible ranges where `.min()` > `.max()` (GX0612), empty `.oneOf()` (GX0610), duplicate `.oneOf()` variants (GX0611), and circular guard dependencies (GX0627).

Guard composition methods: `.partial()` (all fields optional), `.pick("a", "b")` (subset), `.omit("c")` (exclude fields).

### Actions

```text
server {
  action createUser(data: UserForm) -> User {
    let db = connect(env.DATABASE_URL)
    return db.insert("users", data)
  }
}
```

Server-side RPC endpoints. Must be declared inside a `server` block (GX0902). The compiler checks return type serializability (GX0903) and warns if no guard is used (GX0912).

### Queries

```text
client {
  query users = "/api/users" -> User[]
}
```

Client-side reactive data fetching. Must be in a `client` block. The compiler checks URL interpolation types (must be `string` or `int`) and return type deserializability (GX0906).

### Channels

```text
channel notifications(userId: string) -> Notification {
  on connect { ... }
  on receive(msg: Notification) { ... }
  on disconnect { ... }
}
```

WebSocket channels with three direction modes:

- `->` server-to-client (push only)
- `<-` client-to-server (send only)
- `<->` bidirectional

### Stores

```text
store Counter {
  signal count = 0
  derive doubled = count * 2

  fn increment() {
    count += 1
  }

  fn reset() {
    count = 0
  }
}
```

Reactive state containers. The compiler validates: duplicate names (GX1003), stores containing action patterns (GX1004) or query patterns (GX1005), and external signal mutation (GX1002).

### API resources — `out api`

```text
out api Products {
  get[id](id: string) -> Product { ... }
  post(data: CreateProduct) -> Product { ... }
  put[id](id: string, data: UpdateProduct) -> Product { ... }
  delete[id](id: string) -> void { ... }
}
```

REST endpoints for external callers. Supports GET, POST, PUT, PATCH, DELETE with optional path parameters `[id]`. The compiler checks for duplicate handlers, path param types (always `string`), and return type serializability.

### Middleware

```text
middleware requireAuth(req, next) {
  let token = req.header("Authorization")
  when token == null {
    return Response.status(401).json({ error: "Unauthorized" })
  }
  return next(req)
}
```

Request interceptors. Must have exactly 2 parameters (GX1301). The compiler verifies no client-side code (signals, derives, effects) appears in middleware bodies (GX1304).

### Env

```text
env {
  DATABASE_URL: string.nonEmpty()
  PORT: int.default(3000)
  GALE_PUBLIC_API_URL: string
}
```

Typed environment variable declarations. Server-only env vars cannot start with `GALE_PUBLIC_` (GX1103). Client-accessible env vars must start with `GALE_PUBLIC_` (GX1102). The compiler warns if no validation chain is provided (GX1106).

### Enums and type aliases

```text
shared {
  enum Status { Active, Inactive, Pending }
  type UserId = string
}
```

String-like enums and type aliases. `shared` makes them available to both server and client code.

### Functions

```text
fn formatDate(date: string) -> string {
  // ...
}
```

Regular functions with optional `async` modifier.

### Tests

```text
test "user creation" {
  let user = createUser({ name: "Alice" })
  assert user.name == "Alice"
}
```

> **Note:** Test discovery works but the test runner does not yet compile test bodies. Tests are discovered and parsed but always pass vacuously.

## Boundary blocks

```text
server {
  // Server-only code. Cannot be accessed from client scope.
  action save(data: Form) -> string { ... }
}

client {
  // Client-only code. Cannot access server bindings.
  signal count = 0
  derive doubled = count * 2
}

shared {
  // Available to both server and client.
  enum Status { Active, Inactive }
  type UserId = string
}
```

The compiler enforces boundaries at compile time with 24 error codes (GX0500–GX0523):

- **GX0500**: Server binding accessed from client scope
- **GX0501**: Client binding accessed from server scope
- **GX0503**: Non-serializable type crossing boundary
- **GX0510–GX0523**: Export coherence, env visibility, declaration-in-wrong-scope

## Statements

| Statement | Example | Notes |
|-----------|---------|-------|
| `let` | `let x = 42` | Immutable binding |
| `mut` | `mut x = 42` | Mutable binding |
| `signal` | `signal count = 0` | Reactive state (client only, GX1600) |
| `derive` | `derive doubled = count * 2` | Computed from signals (no mutations allowed, GX1605) |
| `frozen` | `frozen config = loadConfig()` | Deep immutable binding |
| `ref` | `ref inputEl: HTMLInputElement` | DOM element reference |
| `if/else` | `if x > 0 { ... } else { ... }` | Condition must be `bool` |
| `for...in` | `for item, index in list { ... }` | Iterable must be `Array` |
| `return` | `return value` | Constrained to function return type |
| `effect` | `effect { ... }` | Side effect with optional cleanup |
| `watch` | `watch count as (next, prev) { ... }` | Must watch a reactive source (Signal or Derived) |

## Template syntax

### Elements

Standard HTML elements with self-closing support:

```text
<div class="container">
  <h1>Title</h1>
  <img src="/logo.png" alt="Logo" />
</div>
```

The compiler warns about void elements with children (GX0708), inline styles (GX0720), and deep nesting beyond a threshold (GX0722).

### Expression interpolation

```text
<p>{user.name}</p>
<p>{count * 2}</p>
```

Interpolated expressions must be renderable (`string`, `int`, `float`, or `bool`).

### Conditionals — `when`

```text
<when loggedIn>
  <p>Welcome, {name}</p>
</when>
<else>
  <p>Please log in</p>
</else>
```

Condition must be `bool`. Supports `else when` chaining.

### Lists — `each`

```text
<each item, index in items>
  <li key={item.id}>{item.name}</li>
</each>
<empty>
  <p>No items found</p>
</empty>
```

Iterable must be an `Array`. The linter warns if `key` is missing (GX1705).

### Directives

| Directive | Example | Type checking |
|-----------|---------|--------------|
| `bind:field` | `<input bind:value={name} />` | Target must be `Signal`; inner type must be compatible with element attribute |
| `on:event` | `<button on:click={handler}>` | Handler must be a function; parameter type checked against DOM event type |
| `on:event.modifier` | `<form on:submit.prevent={...}>` | Modifiers: `prevent`, `stop`, `once`, `self`, `capture` |
| `class:name` | `<div class:active={isActive}>` | Condition must be `bool` |
| `ref:name` | `<input ref:inputEl />` | Target must be `DomRef`; inner type must match element type |
| `key` | `<li key={id}>` | Must be `string` or `int` |
| `transition:type` | `<div transition:fade={config}>` | Config expression is type-inferred |
| `form:action` | `<form form:action={submit}>` | Must be a function/action |
| `form:guard` | `<form form:guard={LoginForm}>` | Must be a guard type |
| `form:error` | `<form:error field="email" />` | Displays validation error for field |
| `into:slot` | `<div into:sidebar>` | Targets a named slot |
| `prefetch` | `<a prefetch="hover">` | Link prefetching mode |

### Slots

```text
// In layout:
<slot/>
<slot name="sidebar"/>

// In page:
<div into:sidebar>Sidebar content</div>
```

## Type system

### Primitive types

`string`, `int`, `float`, `bool`, `void`, `null`, `never`

### Compound types

| Type | Syntax | Example |
|------|--------|---------|
| Array | `T[]` | `string[]`, `int[]` |
| Optional | `T?` | `string?` |
| Union | `T \| U` | `string \| null` |
| Tuple | `(T, U)` | `(string, int)` |
| Object | `{ key: T }` | `{ name: string, age: int }` |
| Function | `fn(T) -> U` | `fn(string) -> bool` |
| String literal | `"value"` | `"primary" \| "ghost"` |

### Type inference

GaleX uses constraint-based type inference with Robinson unification. Types are inferred from:

- Literal values (`42` is `int`, `"hello"` is `string`)
- Operator usage (`a + b` where `a: int` infers `b: int`)
- Function calls (argument types constrain parameter types)
- Signal wrapping (`signal x = 0` creates `Signal<int>`)
- Guard field types (derived from type annotation + validators)

Subtyping rules: `StringLiteral <: string`, `IntLiteral <: int`, `int <: float`, `never <: T`, `null <: T?`, `T <: T | U`.
