# Database

GaleX does not include an ORM or query builder. Database access happens through typed actions, env-driven connection strings, and API resources.

## Configuration

Set your database adapter in `galex.toml`:

```toml
[database]
adapter = "postgres"    # or "sqlite"
```

Declare the connection string as a typed env var:

```text
env {
  DATABASE_URL: string.nonEmpty()
}
```

The compiler generates a typed Rust env module with fail-fast validation on startup.

## Actions as data layer

Actions are the primary way to interact with your database:

```text
server {
  action getUser(id: string) -> User? {
    let db = connect(env.DATABASE_URL)
    return db.query("SELECT * FROM users WHERE id = $1", id)
  }

  action createUser(data: CreateUserForm) -> User {
    let db = connect(env.DATABASE_URL)
    return db.insert("users", data)
  }

  action updateUser(id: string, data: UpdateUserForm) -> User {
    let db = connect(env.DATABASE_URL)
    return db.update("users", id, data)
  }

  action deleteUser(id: string) -> void {
    let db = connect(env.DATABASE_URL)
    db.query("DELETE FROM users WHERE id = $1", id)
  }
}
```

Actions run on the server and are compiled to Axum POST handlers. They can access env vars, call external services, and return typed data.

## API resources for external access

If you need REST endpoints for external callers (mobile apps, third-party integrations), use `out api`:

```text
out api Users {
  get() -> User[] {
    let db = connect(env.DATABASE_URL)
    return db.query("SELECT * FROM users")
  }

  get[id](id: string) -> User {
    let db = connect(env.DATABASE_URL)
    return db.query("SELECT * FROM users WHERE id = $1", id)
  }

  post(data: CreateUserForm) -> User {
    let db = connect(env.DATABASE_URL)
    return db.insert("users", data)
  }
}
```

The compiler generates Axum route handlers with typed extractors (Query, Json, Path) for each HTTP method.

## Validating input with guards

Use guards to validate data before it reaches the database:

```text
guard CreateUserForm {
  name: string.trim().minLen(2).maxLen(100)
  email: string.trim().email()
  role: string.oneOf("user", "admin").default("user")
}
```

The guard's validation runs both client-side (for instant feedback) and server-side (for security). Invalid data never reaches your action.

## Typed queries from the client

Use queries for client-side data fetching:

```text
client {
  query users = "/api/users" -> User[]
  query user = "/api/users/{userId}" -> User
}
```

Queries generate JavaScript fetch wrappers with caching and reactive updates.
