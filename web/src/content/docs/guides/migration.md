# Migration from React / Next.js

A concept mapping for developers coming from the React ecosystem.

## Concept mapping

| React / Next.js | GaleX | Notes |
|----------------|-------|-------|
| `useState` | `signal` | `signal count = 0` creates reactive state |
| `useMemo` | `derive` | `derive doubled = count * 2` auto-updates |
| `useEffect` | `effect` | `effect { ... }` with optional cleanup |
| `useRef` | `ref` | `ref inputEl: HTMLInputElement` |
| Component function | `out ui Name()` | Top-level declaration, not a function call |
| JSX `{expression}` | `{expression}` | Same interpolation syntax |
| JSX `className` | `class` | Standard HTML attribute name |
| `onClick={handler}` | `on:click={handler}` | Directive syntax with modifiers |
| `value={state}` | `bind:value={signal}` | Two-way binding to signals |
| Conditional `{x && <El/>}` | `<when x><El/></when>` | Block-level conditionals |
| `{items.map(i => ...)}` | `<each item in items>...</each>` | Block-level iteration with `empty` fallback |
| `getServerSideProps` | `server { }` block | Server code in the same file |
| API route (`/api/...`) | `out api Name { }` | Typed REST endpoints |
| Server Action | `action name() { }` | Typed RPC, compiled to POST handler |
| Zod schema | `guard Name { }` | Built-in validation with 20 validators |
| `<Suspense>` | `<suspend>` | Async boundaries |
| `<Outlet>` / `{children}` | `<slot/>` | Content injection in layouts |
| `next.config.js` | `galex.toml` | Project configuration |
| `layout.tsx` | `layout.gx` | File-based layout nesting |
| `page.tsx` | `page.gx` | File-based routing |
| `[slug]/page.tsx` | `[slug]/page.gx` | Dynamic route segments |
| `[...rest]/page.tsx` | `[...rest]/page.gx` | Catch-all routes |
| `middleware.ts` | `middleware name(req, next)` | Request interceptors |
| `.env.local` | `env { }` declaration + `.env` file | Typed, validated env vars |
| React Context | `store Name { }` | Reactive singleton state |
| `useReducer` | `store` methods | Mutation methods in store |
| Socket.IO / Pusher | `channel name <-> Type` | Built-in typed WebSocket |

## Template syntax differences

### Conditionals

```text
// React
{isLoggedIn ? <Dashboard /> : <Login />}
{error && <ErrorBanner />}

// GaleX
<when isLoggedIn>
  <Dashboard />
</when>
<else>
  <Login />
</else>

<when error>
  <ErrorBanner />
</when>
```

### Lists

```text
// React
{items.map(item => (
  <li key={item.id}>{item.name}</li>
))}

// GaleX
<each item in items>
  <li key={item.id}>{item.name}</li>
</each>
<empty>
  <p>No items</p>
</empty>
```

### Event handlers

```text
// React
<button onClick={(e) => { e.preventDefault(); handleClick(); }}>

// GaleX
<button on:click.prevent={handleClick}>
```

Modifiers: `.prevent`, `.stop`, `.once`, `.self`, `.capture`.

### Two-way binding

```text
// React
<input value={name} onChange={(e) => setName(e.target.value)} />

// GaleX
<input bind:value={name} />
```

`bind:` requires the target to be a `signal`.

## Key operational differences

| Aspect | React / Next.js | GaleX |
|--------|----------------|-------|
| Runtime | Node.js | Single Rust binary |
| Build output | JS bundles + SSR server | Native binary + minimal JS |
| Client framework | React runtime (~40KB) | No virtual DOM, selective hydration |
| Type checking | TypeScript (optional) | Built-in type system (mandatory) |
| Validation | External (Zod, Yup) | Built-in guards |
| Bundler | Webpack / Turbopack | Compiler generates Rust + JS directly |
| Deployment | Node.js server / Edge | Single binary, any platform |
| Hot reload | Fast Refresh (HMR) | Full rebuild via dev proxy |

## What's different

GaleX is **not** React. Key differences:

1. **No virtual DOM.** Templates compile to server-rendered HTML. Interactive elements hydrate selectively.
2. **No hooks.** Reactivity uses `signal`, `derive`, `effect`, and `watch` at the declaration level, not inside render functions.
3. **Boundaries are physical.** `server { }` and `client { }` blocks are compiler-enforced. You cannot accidentally ship server code to the client.
4. **Validation is built-in.** Guards replace Zod/Yup and generate both client and server validation.
5. **Single binary.** No `node_modules`, no `package.json` (except for Tailwind), no runtime dependencies.
