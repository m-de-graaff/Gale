# Forms

The guard + action + template pattern for validated forms.

## The pattern

```text
guard ContactForm {
  name: string.trim().minLen(2).maxLen(100)
  email: string.trim().email()
  message: string.trim().minLen(10).maxLen(2000)
}

server {
  action submitContact(data: ContactForm) -> string {
    await send_email(data.email, data.message)
    return "Sent"
  }
}

client {
  signal result = ""
}
```

Wire them together with `form:guard` and `form:action`:

```text
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
  <button type="submit">Send</button>
  when result != "" {
    <p>{result}</p>
  }
</form>
```

## What the compiler generates

**Server (Rust):** A struct with `#[derive(Debug, Clone, Serialize, Deserialize)]`, a `validate()` method, and an Axum POST handler.

**Client (JavaScript):** A mirror validation function, form wiring (`getFormData`, `clearErrors`, `showErrors`), and a fetch-based action stub.

## Compiler checks

- `form:action` without `form:guard` — GX1505 (warning)
- `.min(10).max(5)` — GX0603: min exceeds max
- `.oneOf()` with no values — GX0619
- `.oneOf("a", "a")` — GX0620: duplicate variant
- `.email()` on `int` — GX0605: string-only validator on wrong type

## Guard composition

```text
let partial = FullProfile.partial()     // All fields optional
let subset = FullProfile.pick("email")  // Only listed fields
let without = FullProfile.omit("bio")   // Exclude fields
```
