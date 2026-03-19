# Env

Typed environment variable declarations.

```text
env {
  DATABASE_URL: string.nonEmpty()
  PORT: int.default(3000)
  JWT_SECRET: string.minLen(32)
  GALE_PUBLIC_API_URL: string
}
```

## Visibility rules

- Server-only env vars are inaccessible from client code
- Client-accessible vars **must** start with `GALE_PUBLIC_` (GX1102)
- Server vars **should not** start with `GALE_PUBLIC_` (GX1103)
- Only primitive types allowed (`string`, `int`, `float`, `bool`)

## Generated code

The compiler generates a typed Rust env config module with:

- `dotenvy` for `.env` file loading
- Fail-fast validation on startup (missing required vars cause exit)
- A `public_vars_json()` function that exposes only `PUBLIC_`-prefixed vars to client code
- A `LazyLock` singleton for the env struct

## Compiler checks

| Code | Condition |
|------|-----------|
| GX1102 | Client env missing `GALE_PUBLIC_` prefix |
| GX1103 | Server env has `GALE_PUBLIC_` prefix (warning) |
| GX1104 | Duplicate env variable across sections |
| GX1106 | No validation chain (warning) |
| GX0517 | Server env accessed in client block |
