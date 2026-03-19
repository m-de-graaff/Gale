# Actions

Server-side RPC endpoints declared inside `server` blocks.

```text
server {
  action createPost(data: PostForm) -> Post {
    let db = connect(env.DATABASE_URL)
    return db.insert("posts", data)
  }
}
```

Actions are compiled to Axum POST handlers at `/api/__gx/actions/{name}`. The compiler generates:

- **Rust:** POST handler with guard deserialization and validation
- **JavaScript:** Client-side RPC stub with `fetch()`, typed error classes (`GaleValidationError`, `GaleServerError`, `GaleNetworkError`), and query cache integration

## Compiler checks

| Code | Condition |
|------|-----------|
| GX0512 | `action` declared in client block |
| GX0516 | `action` declared in shared block |
| GX0902 | Action must be in server scope |
| GX0903 | Return type must be serializable |
| GX0904 | Duplicate action name |
| GX0912 | No guard parameter (warning) |
