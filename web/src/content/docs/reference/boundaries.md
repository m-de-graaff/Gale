# Boundaries

Server, client, and shared scopes — enforced at compile time.

```text
server {
  let secret = env.API_SECRET
  action charge(amount: int) -> bool {
    return process_payment(env.STRIPE_KEY, amount)
  }
}

client {
  signal count = 0
  derive doubled = count * 2
  // let x = secret   -> GX0500: Cannot access server binding
}

shared {
  enum Status { Active, Inactive }
  type UserId = string
}
```

## Rules

- **server** — code runs only on the server. Actions, database calls, env secrets.
- **client** — code runs in the browser. Signals, derives, effects, event handlers.
- **shared** — available to both. Enums, type aliases, pure functions only.

## Implicit boundary mapping

Some declarations have implicit scopes:

| Declaration | Implicit scope |
|-------------|---------------|
| `action` | Server |
| `query` | Client |
| `middleware` | Server |
| `signal`, `derive`, `ref` | Client |
| `guard`, `enum`, `type` | Shared |

## 24 boundary error codes (GX0500–GX0523)

| Code | Condition |
|------|-----------|
| GX0500 | Server binding accessed from client |
| GX0501 | Client binding accessed from server |
| GX0502 | Server binding accessed from shared |
| GX0503 | Client binding accessed from shared |
| GX0504 | Non-serializable type crossing boundary |
| GX0507 | `signal` in server block |
| GX0508 | `derive` in server block |
| GX0509 | `effect` in server block |
| GX0510 | `ref` in server block |
| GX0511 | `query` in server block |
| GX0512 | `action` in client block |
| GX0513 | `channel` in client block |
| GX0517 | Server env accessed in client block |
| GX0521 | `out` in server block |
| GX0523 | `bind` directive references server binding |

Non-serializable types that cannot cross boundaries: functions, signals, stores, channels, DOM refs.
