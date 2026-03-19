# Middleware

Request interceptors applied to routes.

```text
middleware requireAuth(req, next) {
  let token = req.header("Authorization")
  when token == null {
    return Response.status(401).json({ error: "Unauthorized" })
  }
  return next(req)
}
```

Middleware must have exactly 2 parameters — `req` and `next` (GX1301).

## Targeting

The parser supports a `for` clause for targeting specific routes:

```text
middleware for "/api" requireAuth(req, next) {
  // Only applies to /api/* routes
  return next(req)
}
```

Middleware targets:

- **Global** — applied to all routes
- **Path prefix** — applied to routes under a path segment
- **Resource** — applied to a specific API resource

## Compiler checks

| Code | Condition |
|------|-----------|
| GX1301 | Not exactly 2 parameters |
| GX1304 | Client-side code in middleware body (signal, derive, effect, watch, ref) |

## Generated code

The compiler generates Axum middleware layer functions with expression rewriting for `next()` calls, `Response.status()`, and `Response.json()`. A `GaleRequest`/`GaleResponse` runtime provides `header()`, `status()`, `path()`, `method()`, `set_header()`, `json()`, and `redirect()` methods.
