# Authentication

Authentication in GaleX uses middleware for route protection, env for secrets, and actions for login/logout logic.

## Login action with guard

```text
guard LoginForm {
  email: string.trim().email()
  password: string.minLen(8).maxLen(128)
}

server {
  action login(data: LoginForm) -> Session {
    let db = connect(env.DATABASE_URL)
    let user = db.query("SELECT * FROM users WHERE email = $1", data.email)

    when user == null {
      return { error: "Invalid credentials" }
    }

    let valid = verify_hash(user.password_hash, data.password)
    when !valid {
      return { error: "Invalid credentials" }
    }

    return create_session(user.id)
  }

  action logout() -> void {
    destroy_session()
  }
}
```

## Middleware for route protection

```text
middleware requireAuth(req, next) {
  let token = req.header("Authorization")
  when token == null {
    return Response.status(401).json({ error: "Unauthorized" })
  }

  let user = verify_jwt(token, env.JWT_SECRET)
  when user == null {
    return Response.status(403).json({ error: "Forbidden" })
  }

  return next(req)
}
```

Middleware must have exactly 2 parameters (GX1301). The compiler verifies no client-side constructs (signals, derives, effects) appear in middleware bodies (GX1304).

## Typed env for secrets

```text
env {
  JWT_SECRET: string.minLen(32)
  DATABASE_URL: string.nonEmpty()
  SESSION_TTL: int.default(3600)
}
```

Server-only env vars are inaccessible from client code. The `GALE_PUBLIC_` prefix convention makes specific vars available to client code (GX1102/GX1103).

The compiler generates a typed Rust struct for env vars with fail-fast validation on startup via `dotenvy`.

## Session management pattern

GaleX does not have a built-in session store. Use actions to create and destroy sessions:

```text
server {
  action login(data: LoginForm) -> Session {
    // Your session creation logic
    let session = create_session(user.id)
    return session
  }

  action logout() -> void {
    // Your session destruction logic
    destroy_session()
  }

  action getCurrentUser() -> User? {
    // Your session verification logic
    let session = get_session()
    when session == null { return null }
    return find_user(session.user_id)
  }
}
```

## Combining with forms

```text
<form form:action={login} form:guard={LoginForm}>
  <input bind:value={email} type="email" placeholder="Email" />
  <form:error field="email" />

  <input bind:value={password} type="password" placeholder="Password" />
  <form:error field="password" />

  <button type="submit">Sign in</button>
</form>
```

The guard validates the form client-side (instant feedback) and server-side (security). The action handles the actual authentication logic.
