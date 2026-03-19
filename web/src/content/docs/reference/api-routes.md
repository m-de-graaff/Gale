# API Routes

Stable HTTP endpoints for external callers.

```text
out api Products {
  get() -> Product[] {
    return db.query("SELECT * FROM products")
  }

  get[id](id: string) -> Product {
    return db.query("SELECT * FROM products WHERE id = $1", id)
  }

  post(data: CreateProduct) -> Product {
    return db.insert("products", data)
  }

  delete[id](id: string) -> void {
    db.query("DELETE FROM products WHERE id = $1", id)
  }
}
```

Supports `get`, `post`, `put`, `patch`, `delete`. Path parameters `[id]` are always typed as `string`.

## Code generation

The compiler generates Axum route handlers with typed extractors:

- **No params:** handler with no extractors
- **Path params:** `Path<String>` extractor
- **Body params:** `Json<T>` extractor with guard struct
- **Status codes:** 200 (GET/PUT/PATCH), 201 (POST), 204 (DELETE), 422 (validation failure)

## Compiler checks

The compiler detects duplicate handlers per HTTP method and validates return type serializability.
