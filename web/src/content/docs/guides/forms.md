# Forms

The guard + action + template pattern for validated forms.

## The pattern

Every form in GaleX connects three pieces:

1. **Guard** — defines the validation schema
2. **Action** — handles the server-side submission
3. **Template** — wires them together with `form:guard` and `form:action` directives

```text
guard ContactForm {
  name: string.trim().minLen(2).maxLen(100)
  email: string.trim().email()
  subject: string.trim().minLen(5)
  message: string.trim().minLen(10).maxLen(2000)
}

server {
  action submitContact(data: ContactForm) -> string {
    await send_email(data.email, data.subject, data.message)
    return "Message sent successfully"
  }
}

client {
  signal result = ""
}

<form form:action={submitContact} form:guard={ContactForm}>
  <label>
    Name
    <input bind:value={name} />
    <form:error field="name" />
  </label>

  <label>
    Email
    <input bind:value={email} type="email" />
    <form:error field="email" />
  </label>

  <label>
    Subject
    <input bind:value={subject} />
    <form:error field="subject" />
  </label>

  <label>
    Message
    <textarea bind:value={message} />
    <form:error field="message" />
  </label>

  <button type="submit">Send</button>

  <when result != "">
    <p>{result}</p>
  </when>
</form>
```

## What the compiler generates

**Server (Rust):**
- A struct matching the guard fields with `Deserialize`
- A `validate()` method implementing all validator chains
- An Axum POST handler that deserializes, validates, and calls the action

**Client (JavaScript):**
- A mirror validation function for instant client-side feedback
- Form wiring: `getFormData()`, `clearErrors()`, `showErrors()`
- A fetch-based action stub with error handling classes

## Validation chains

Validators compose left-to-right. Transforms (`.trim()`, `.precision()`) run before validators:

```text
guard SignupForm {
  // Transform then validate
  username: string.trim().minLen(3).maxLen(20)

  // Multiple validators
  age: int.min(13).max(150).positive()

  // Optional with default
  newsletter: bool.optional().default(false)
}
```

The compiler detects impossible configurations:

- `.min(10).max(5)` — GX0612: min exceeds max
- `.oneOf()` with no values — GX0610
- `.oneOf("a", "a")` — GX0611: duplicate variant
- `.email()` on `int` — GX0607: incompatible type

## Guard composition

Derive new guards from existing ones:

```text
guard FullProfile {
  name: string.trim().minLen(2)
  email: string.trim().email()
  bio: string.maxLen(500)
  avatar: string.url()
}

// All fields become optional
let editProfile = FullProfile.partial()

// Only specific fields
let loginFields = FullProfile.pick("email")

// Exclude fields
let publicProfile = FullProfile.omit("email")
```

## Compiler checks for forms

The compiler warns if `form:action` is used without `form:guard` (GX1505). The type checker cross-validates that the guard type is compatible with the action's parameter type.

The linter also checks for:
- Missing labels on form inputs (GX1710)
- Missing `alt` on img elements (GX1706)
