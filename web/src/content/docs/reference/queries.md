# Queries

Client-side reactive data fetching.

```text
client {
  query posts = "/api/posts" -> Post[]
  query user = "/api/users/{userId}" -> User
}
```

URL interpolation variables must be `string` or `int`. The compiler generates JavaScript fetch wrappers with caching, revalidation, and retry support.

## Compiler checks

| Code | Condition |
|------|-----------|
| GX0511 | `query` declared in server block |
| GX0906 | Return type not deserializable |
| GX0913 | Query in wrong scope |
