# Authentication

Middleware for route protection, env for secrets, actions for login logic.

## Login action

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
}
```

## Middleware

```text
middleware requireAuth(req, next) {
  let token = req.header("Authorization")
  when token == null {
    return Response.status(401).json({ error: "Unauthorized" })
  }
  return next(req)
}
```

Must have exactly 2 parameters (GX1301). No client code allowed (GX1304).

## Typed env

```text
env {
  JWT_SECRET: string.minLen(32)
  DATABASE_URL: string.nonEmpty()
  GALE_PUBLIC_API_URL: string
}
```

Server vars are inaccessible from client. `GALE_PUBLIC_` prefix makes vars client-accessible (GX1102/GX1103).

## Form integration

```text
<form form:action={login} form:guard={LoginForm}>
  <input bind:value={email} type="email" />
  <form:error field="email" />
  <input bind:value={password} type="password" />
  <form:error field="password" />
  <button type="submit">Sign in</button>
</form>
```
