# Migration from React / Next.js

## Concept mapping

| React / Next.js | GaleX | Notes |
|----------------|-------|-------|
| `useState` | `signal` | `signal count = 0` |
| `useMemo` | `derive` | `derive doubled = count * 2` |
| `useEffect` | `effect` | With optional cleanup |
| `useRef` | `ref` | `ref el: HTMLInputElement` |
| Component | `out ui Name()` | Top-level declaration |
| `className` | `class` | Standard HTML attribute |
| `onClick={fn}` | `on:click={fn}` | Directive with modifiers |
| `value={state}` | `bind:value={signal}` | Two-way binding |
| `{x && <El/>}` | `when x { <El/> }` | Curly-brace blocks |
| `.map(...)` | `each item in list { }` | With `empty` fallback |
| Server Action | `action name() { }` | Compiled to POST |
| Zod schema | `guard Name { }` | 28 chain methods |
| `<Suspense>` | `suspend { }` | Async boundary |
| `{children}` | `<slot/>` | Content injection |
| `middleware.ts` | `middleware name(req, next)` | Request interceptor |
| React Context | `store Name { }` | Reactive singleton |
| Socket.IO | `channel name <-> T` | Built-in WebSocket |

## Template syntax

GaleX uses **curly braces**, not angle brackets, for control flow:

```text
when isLoggedIn {
  <Dashboard />
} else {
  <Login />
}

each item in items {
  <li key={item.id}>{item.name}</li>
} empty {
  <p>No items</p>
}
```

Event modifiers: `<button on:click.prevent={handler}>` — `.prevent`, `.stop`, `.once`, `.self`.

Two-way binding: `<input bind:value={name} />` — target must be a `signal`.

## Key differences

| Aspect | React / Next.js | GaleX |
|--------|----------------|-------|
| Runtime | Node.js | Single Rust binary |
| Client | React (~40KB) | No virtual DOM, selective hydration |
| Types | TypeScript (optional) | Built-in (mandatory) |
| Validation | Zod, Yup | Built-in guards |
| Deploy | Node server | Single binary |
